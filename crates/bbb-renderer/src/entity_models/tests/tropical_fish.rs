use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn tropical_fish_small_cubes_match_vanilla_26_1_body_layer() {
    // Vanilla `TropicalFishSmallModel.createBodyLayer` (kob body, atlas 32×32,
    // `CubeDeformation.NONE`, so each `uv_size` equals the geometry size): body, tail, right fin
    // (`yRot = π/4`), left fin (`yRot = -π/4`), top fin. Each unified cube carries the colored tint
    // (`TROPICAL_FISH_ORANGE`) and the textured UV in one struct.
    assert_eq!(
        TROPICAL_FISH_SMALL_BODY[0],
        ModelCube::new(
            [-1.0, -1.5, -3.0],
            [2.0, 3.0, 6.0],
            TROPICAL_FISH_ORANGE,
            [2.0, 3.0, 6.0],
            [0.0, 0.0],
            false,
        )
    );
    // The tail is a zero-thickness plane keeping its negative `texOffs` V origin.
    assert_eq!(TROPICAL_FISH_SMALL_TAIL[0].size, [0.0, 3.0, 6.0]);
    assert_eq!(TROPICAL_FISH_SMALL_TAIL[0].tex, [22.0, -6.0]);
    // The side fins are zero-thickness planes flat in Z.
    assert_eq!(TROPICAL_FISH_SMALL_RIGHT_FIN[0].size, [2.0, 2.0, 0.0]);
    assert_eq!(TROPICAL_FISH_SMALL_TOP_FIN[0].size, [0.0, 3.0, 6.0]);
    assert_eq!(TROPICAL_FISH_SMALL_TOP_FIN[0].tex, [10.0, -5.0]);
}

#[test]
fn tropical_fish_small_part_poses_match_vanilla() {
    // The side fins splay ±π/4 about Y (not Z like cod/salmon); the tail is swayed by `setupAnim`.
    assert_close3(TROPICAL_FISH_SMALL_BODY_POSE.offset, [0.0, 22.0, 0.0]);
    assert_eq!(TROPICAL_FISH_SMALL_BODY_POSE.rotation, [0.0, 0.0, 0.0]);
    assert_close3(TROPICAL_FISH_SMALL_TAIL_POSE.offset, [0.0, 22.0, 3.0]);
    assert_eq!(TROPICAL_FISH_SMALL_TAIL_POSE.rotation, [0.0, 0.0, 0.0]);
    assert_close3(TROPICAL_FISH_SMALL_RIGHT_FIN_POSE.offset, [-1.0, 22.5, 0.0]);
    assert_close3(
        TROPICAL_FISH_SMALL_RIGHT_FIN_POSE.rotation,
        [0.0, std::f32::consts::FRAC_PI_4, 0.0],
    );
    assert_close3(TROPICAL_FISH_SMALL_LEFT_FIN_POSE.offset, [1.0, 22.5, 0.0]);
    assert_close3(
        TROPICAL_FISH_SMALL_LEFT_FIN_POSE.rotation,
        [0.0, -std::f32::consts::FRAC_PI_4, 0.0],
    );
    assert_close3(TROPICAL_FISH_SMALL_TOP_FIN_POSE.offset, [0.0, 20.5, -3.0]);
    assert_eq!(TROPICAL_FISH_SMALL_TOP_FIN_POSE.rotation, [0.0, 0.0, 0.0]);
}

#[test]
fn tropical_fish_large_cubes_match_vanilla_26_1_body_layer() {
    // Vanilla `TropicalFishLargeModel.createBodyLayer` (flopper body, atlas 32×32): body, tail,
    // right fin, left fin, top fin, bottom fin.
    assert_eq!(
        TROPICAL_FISH_LARGE_BODY[0],
        ModelCube::new(
            [-1.0, -3.0, -3.0],
            [2.0, 6.0, 6.0],
            TROPICAL_FISH_ORANGE,
            [2.0, 6.0, 6.0],
            [0.0, 20.0],
            false,
        )
    );
    assert_eq!(TROPICAL_FISH_LARGE_TAIL[0].size, [0.0, 6.0, 5.0]);
    assert_eq!(TROPICAL_FISH_LARGE_TAIL[0].tex, [21.0, 16.0]);
    assert_eq!(TROPICAL_FISH_LARGE_TOP_FIN[0].size, [0.0, 4.0, 6.0]);
    // The bottom fin is unique to the large (flopper) body.
    assert_eq!(TROPICAL_FISH_LARGE_BOTTOM_FIN[0].size, [0.0, 4.0, 6.0]);
    assert_eq!(TROPICAL_FISH_LARGE_BOTTOM_FIN[0].tex, [20.0, 21.0]);
}

#[test]
fn tropical_fish_large_part_poses_match_vanilla() {
    assert_close3(TROPICAL_FISH_LARGE_BODY_POSE.offset, [0.0, 19.0, 0.0]);
    assert_close3(TROPICAL_FISH_LARGE_TAIL_POSE.offset, [0.0, 19.0, 3.0]);
    assert_close3(TROPICAL_FISH_LARGE_RIGHT_FIN_POSE.offset, [-1.0, 20.0, 0.0]);
    assert_close3(
        TROPICAL_FISH_LARGE_RIGHT_FIN_POSE.rotation,
        [0.0, std::f32::consts::FRAC_PI_4, 0.0],
    );
    assert_close3(TROPICAL_FISH_LARGE_LEFT_FIN_POSE.offset, [1.0, 20.0, 0.0]);
    assert_close3(
        TROPICAL_FISH_LARGE_LEFT_FIN_POSE.rotation,
        [0.0, -std::f32::consts::FRAC_PI_4, 0.0],
    );
    assert_close3(TROPICAL_FISH_LARGE_TOP_FIN_POSE.offset, [0.0, 16.0, -3.0]);
    assert_close3(
        TROPICAL_FISH_LARGE_BOTTOM_FIN_POSE.offset,
        [0.0, 22.0, -3.0],
    );
}

#[test]
fn tropical_fish_tail_sway_matches_vanilla_setup_anim() {
    // Both layers: `tail.yRot = -amplitude * 0.45 * sin(0.6 * ageInTicks)`, amplitude 1.0
    // in water / 1.5 out (identical to `CodModel.setupAnim`). Zero at age 0.
    assert_eq!(tropical_fish_tail_yrot(0.0, true), 0.0);
    assert_eq!(tropical_fish_tail_yrot(0.0, false), 0.0);

    let age = 5.0_f32;
    let s = (0.6 * age).sin();
    assert!((tropical_fish_tail_yrot(age, true) - (-1.0 * 0.45 * s)).abs() < 1.0e-6);
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
fn tropical_fish_small_mesh_uses_vanilla_geometry() {
    // Five cubes → 30 faces / 120 vertices.
    let fish = entity_model_mesh(&[EntityModelInstance::tropical_fish(
        800,
        [0.0, 64.0, 0.0],
        0.0,
        TropicalFishModelShape::Small,
        EntityDyeColor::Orange,
        TropicalFishPattern::Kob,
        EntityDyeColor::White,
    )
    .with_in_water(true)]);
    // The colored debug path emits only the base body (the pattern overlay is texture-only):
    // five cubes → 30 faces / 120 vertices, regardless of the pattern.
    assert_eq!(fish.opaque_faces, 30);
    assert_eq!(fish.vertices.len(), 120);
    assert_eq!(fish.indices.len(), 180);
    // The base body is tinted by `getModelTint = state.baseColor` (the base dye's texture
    // diffuse color), not the grayscale texture's own color.
    assert!(fish
        .vertices
        .iter()
        .any(|vertex| vertex.color
            == shade_color(EntityDyeColor::Orange.texture_diffuse_color(), 1.0)));
}

#[test]
fn tropical_fish_large_mesh_uses_vanilla_geometry() {
    // Six cubes → 36 faces / 144 vertices.
    let fish = entity_model_mesh(&[EntityModelInstance::tropical_fish(
        801,
        [0.0, 64.0, 0.0],
        0.0,
        TropicalFishModelShape::Large,
        EntityDyeColor::White,
        TropicalFishPattern::Flopper,
        EntityDyeColor::White,
    )
    .with_in_water(true)]);
    assert_eq!(fish.opaque_faces, 36);
    assert_eq!(fish.vertices.len(), 144);
    assert_eq!(fish.indices.len(), 216);
}

#[test]
fn tropical_fish_flops_when_out_of_water() {
    // `TropicalFishRenderer.setupRotations` lays a beached fish on its side (`RotZ(90)` +
    // offset). At age 0 the swim wiggle and tail sway are both zero, so the only difference
    // is the flop, which reorients the body.
    let base = EntityModelInstance::tropical_fish(
        802,
        [0.0, 64.0, 0.0],
        0.0,
        TropicalFishModelShape::Small,
        EntityDyeColor::White,
        TropicalFishPattern::Kob,
        EntityDyeColor::White,
    );
    let swimming = entity_model_mesh(&[base.with_in_water(true)]);
    let beached = entity_model_mesh(&[base.with_in_water(false)]);
    assert_eq!(swimming.vertices.len(), beached.vertices.len());
    assert_ne!(
        swimming.vertices, beached.vertices,
        "the beached fish flops"
    );

    let (swim_min, swim_max) = mesh_extents(&swimming);
    let (beach_min, beach_max) = mesh_extents(&beached);
    assert!(
        (swim_max[1] - swim_min[1]) > (swim_max[0] - swim_min[0]),
        "an upright tropical fish is taller than it is wide"
    );
    assert!(
        (beach_max[0] - beach_min[0]) > (beach_max[1] - beach_min[1]),
        "a beached tropical fish is wider than it is tall"
    );
}

#[test]
fn tropical_fish_sways_its_tail_with_age() {
    // A still fish (age 0) is inert; advancing the age sways the tail and wiggles the body.
    let base = EntityModelInstance::tropical_fish(
        803,
        [0.0, 64.0, 0.0],
        0.0,
        TropicalFishModelShape::Large,
        EntityDyeColor::White,
        TropicalFishPattern::Flopper,
        EntityDyeColor::White,
    )
    .with_in_water(true);
    let still = entity_model_mesh(&[base]);
    let swimming = entity_model_mesh(&[base.with_age_in_ticks(7.0)]);
    assert_eq!(still.vertices.len(), swimming.vertices.len());
    assert_ne!(still.vertices, swimming.vertices, "the tail sways with age");
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
fn tropical_fish_shape_from_vanilla_packed_variant() {
    // `TropicalFish.getPattern(packed).base()`: pattern id is the low 16 bits, packed as
    // `base.id | index << 8`. The default variant `0` (KOB/white/white) is the small body.
    assert_eq!(
        TropicalFishModelShape::from_vanilla_packed_variant(0),
        TropicalFishModelShape::Small
    );
    // FLOPPER = LARGE base (id 1), index 0 → packed pattern `0x0001`. With color bytes set
    // (base/pattern color in the high bytes) the shape decode ignores them.
    assert_eq!(
        TropicalFishModelShape::from_vanilla_packed_variant(0x0405_0001),
        TropicalFishModelShape::Large
    );
    // CLAYFISH = LARGE base, index 5 → `0x0501`; BETTY index 4 → `0x0401`.
    assert_eq!(
        TropicalFishModelShape::from_vanilla_packed_variant(0x0501),
        TropicalFishModelShape::Large
    );
    // SPOTTY = SMALL base, index 5 → `0x0500` stays small.
    assert_eq!(
        TropicalFishModelShape::from_vanilla_packed_variant(0x0500),
        TropicalFishModelShape::Small
    );
    // An out-of-range index (6) is not a valid pattern, so `byId` falls back to KOB (small).
    assert_eq!(
        TropicalFishModelShape::from_vanilla_packed_variant(0x0601),
        TropicalFishModelShape::Small
    );
}

#[test]
fn tropical_fish_texture_ref_matches_vanilla_renderer() {
    // `TropicalFishRenderer` keys the small body on `tropical_a` and the large on
    // `tropical_b`; the model layers are `ModelLayers.TROPICAL_FISH_{SMALL,LARGE}`.
    let small = EntityModelKind::TropicalFish {
        shape: TropicalFishModelShape::Small,
        base_color: EntityDyeColor::White,
        pattern: TropicalFishPattern::Kob,
        pattern_color: EntityDyeColor::White,
    };
    let large = EntityModelKind::TropicalFish {
        shape: TropicalFishModelShape::Large,
        base_color: EntityDyeColor::White,
        pattern: TropicalFishPattern::Flopper,
        pattern_color: EntityDyeColor::White,
    };
    assert_eq!(small.model_key(), "tropical_fish_small");
    assert_eq!(large.model_key(), "tropical_fish_large");
    assert_eq!(
        small.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/fish/tropical_a.png",
            size: [32, 32],
        })
    );
    assert_eq!(
        large.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/fish/tropical_b.png",
            size: [32, 32],
        })
    );
}

#[test]
fn tropical_fish_textured_layer_passes_match_vanilla_renderer() {
    // Each body shape renders a cutout base layer (`#main`) plus the `TropicalFishPatternLayer`
    // overlay (`#pattern`, the body mesh inflated by `FISH_PATTERN_DEFORMATION`). The base is
    // tinted by `getModelTint = state.baseColor`, the overlay by `state.patternColor`. The vestigial
    // `parts` slices are nulled — emit builds the unified model and renders its tree.
    for (shape, layer, texture, pattern, pattern_layer, pattern_texture) in [
        (
            TropicalFishModelShape::Small,
            "minecraft:tropical_fish_small#main",
            TROPICAL_FISH_SMALL_TEXTURE_REF,
            TropicalFishPattern::Brinely,
            "minecraft:tropical_fish_small#pattern",
            TROPICAL_FISH_BRINELY_PATTERN_TEXTURE_REF,
        ),
        (
            TropicalFishModelShape::Large,
            "minecraft:tropical_fish_large#main",
            TROPICAL_FISH_LARGE_TEXTURE_REF,
            TropicalFishPattern::Betty,
            "minecraft:tropical_fish_large#pattern",
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

        assert_eq!(passes[0].kind, EntityModelLayerKind::TropicalFishBase);
        assert_eq!(
            passes[0].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
        assert_eq!(passes[0].model_layer, layer);
        assert_eq!(passes[0].texture, texture);
        // `getModelTint = state.baseColor`: the base layer is tinted by the base dye's diffuse
        // color, not left white.
        assert_eq!(
            passes[0].tint,
            EntityDyeColor::Orange.texture_diffuse_color()
        );
        assert_eq!((passes[0].order, passes[0].submit_sequence), (0, 0));

        assert_eq!(passes[1].kind, EntityModelLayerKind::TropicalFishPattern);
        assert_eq!(
            passes[1].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[1].render_type.vanilla_name(), "entityCutout");
        assert_eq!(passes[1].model_layer, pattern_layer);
        assert_eq!(passes[1].texture, pattern_texture);
        // The overlay is tinted by the pattern dye's diffuse color and drawn after the base.
        assert_eq!(passes[1].tint, EntityDyeColor::Cyan.texture_diffuse_color());
        assert_eq!((passes[1].order, passes[1].submit_sequence), (1, 1));
    }

    // The base layer uses `CubeDeformation.NONE` (no mirror, `uv_size == size`).
    assert!(!TROPICAL_FISH_SMALL_BODY[0].mirror);
    assert_eq!(
        TROPICAL_FISH_SMALL_BODY[0].uv_size,
        TROPICAL_FISH_SMALL_BODY[0].size
    );
    // The small tail/top fin keep their negative `texOffs` V origins.
    assert_eq!(TROPICAL_FISH_SMALL_TAIL[0].tex, [22.0, -6.0]);
    assert_eq!(TROPICAL_FISH_SMALL_TOP_FIN[0].tex, [10.0, -5.0]);
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
    .with_in_water(true)
    .with_light_coords((6_u32 << 4) | (12_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true);
    let small_still = entity_model_textured_meshes(&[small], &atlas);
    assert_tropical_fish_submission_pair(
        &small_still,
        small,
        true,
        TROPICAL_FISH_SMALL_TEXTURE_REF,
        EntityDyeColor::Orange.texture_diffuse_color(),
        TROPICAL_FISH_BRINELY_PATTERN_TEXTURE_REF,
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
    .with_in_water(true)
    .with_light_coords((6_u32 << 4) | (12_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true);
    let large_still = entity_model_textured_meshes(&[large], &atlas);
    assert_tropical_fish_submission_pair(
        &large_still,
        large,
        true,
        TROPICAL_FISH_LARGE_TEXTURE_REF,
        EntityDyeColor::Orange.texture_diffuse_color(),
        TROPICAL_FISH_BETTY_PATTERN_TEXTURE_REF,
        EntityDyeColor::Cyan.texture_diffuse_color(),
    );
    assert_eq!(large_still.cutout.vertices.len(), 288);

    // The tail sway / body wiggle reorient the mesh as the age advances.
    let swimming_instance = small.with_age_in_ticks(7.0);
    let swimming = entity_model_textured_meshes(&[swimming_instance], &atlas);
    assert_tropical_fish_submission_pair(
        &swimming,
        swimming_instance,
        true,
        TROPICAL_FISH_SMALL_TEXTURE_REF,
        EntityDyeColor::Orange.texture_diffuse_color(),
        TROPICAL_FISH_BRINELY_PATTERN_TEXTURE_REF,
        EntityDyeColor::Cyan.texture_diffuse_color(),
    );
    assert_eq!(
        small_still.cutout.vertices.len(),
        swimming.cutout.vertices.len()
    );
    assert_ne!(small_still.cutout.vertices, swimming.cutout.vertices);

    // A beached fish flops onto its side.
    let beached_instance = small.with_in_water(false);
    let beached = entity_model_textured_meshes(&[beached_instance], &atlas);
    assert_tropical_fish_submission_pair(
        &beached,
        beached_instance,
        false,
        TROPICAL_FISH_SMALL_TEXTURE_REF,
        EntityDyeColor::Orange.texture_diffuse_color(),
        TROPICAL_FISH_BRINELY_PATTERN_TEXTURE_REF,
        EntityDyeColor::Cyan.texture_diffuse_color(),
    );
    assert_ne!(small_still.cutout.vertices, beached.cutout.vertices);
}

#[test]
fn tropical_fish_pattern_submission_survives_missing_texture_atlas_entry() {
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
        .with_in_water(true)
        .with_light_coords((6_u32 << 4) | (12_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
        let meshes = entity_model_textured_meshes(&[instance], &atlas);

        assert_eq!(meshes.submissions.len(), 2);
        let base = meshes.submissions[0];
        assert_eq!(base.render_type, EntityModelLayerRenderType::EntityCutout);
        assert_eq!(base.render_type.vanilla_name(), "entityCutout");
        assert_eq!(base.texture, base_texture);
        let base_tint = EntityDyeColor::Orange.texture_diffuse_color();
        assert_eq!(base.tint, base_tint);
        assert_eq!(
            base.transform,
            tropical_fish_model_root_transform(instance, true)
        );
        assert_eq!(base.light, instance.render_state.shader_light());
        assert_eq!(base.overlay, instance.render_state.overlay_coords());
        assert_eq!((base.order, base.submit_sequence), (0, 0));

        let pattern_submit = meshes.submissions[1];
        assert_eq!(
            pattern_submit.render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(pattern_submit.render_type.vanilla_name(), "entityCutout");
        assert_eq!(pattern_submit.texture, pattern_texture);
        let pattern_tint = EntityDyeColor::Cyan.texture_diffuse_color();
        assert_eq!(pattern_submit.tint, pattern_tint);
        assert_eq!(pattern_submit.transform, base.transform);
        assert_eq!(pattern_submit.light, instance.render_state.shader_light());
        assert_eq!(
            pattern_submit.overlay,
            [0.0, instance.render_state.overlay_coords()[1]]
        );
        assert_ne!(
            pattern_submit.overlay,
            instance.render_state.overlay_coords()
        );
        assert_eq!(
            (pattern_submit.order, pattern_submit.submit_sequence),
            (1, 1)
        );

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
                && vertex.light == base.light
                && vertex.overlay == base.overlay));
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
    in_water: bool,
    base_texture: EntityModelTextureRef,
    base_tint: [f32; 4],
    pattern_texture: EntityModelTextureRef,
    pattern_tint: [f32; 4],
) {
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.submissions.len(), 2);

    let base = meshes.submissions[0];
    assert_eq!(base.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(base.render_type.vanilla_name(), "entityCutout");
    assert_eq!(base.texture, base_texture);
    assert_eq!(base.tint, base_tint);
    assert_eq!(
        base.transform,
        tropical_fish_model_root_transform(instance, in_water)
    );
    assert_eq!(base.light, instance.render_state.shader_light());
    assert_eq!(base.overlay, instance.render_state.overlay_coords());
    assert_eq!((base.order, base.submit_sequence), (0, 0));

    let pattern = meshes.submissions[1];
    assert_eq!(
        pattern.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(pattern.render_type.vanilla_name(), "entityCutout");
    assert_eq!(pattern.texture, pattern_texture);
    assert_eq!(pattern.tint, pattern_tint);
    assert_eq!(pattern.transform, base.transform);
    assert_eq!(pattern.light, instance.render_state.shader_light());
    assert_eq!(
        pattern.overlay,
        [0.0, instance.render_state.overlay_coords()[1]]
    );
    assert_ne!(pattern.overlay, instance.render_state.overlay_coords());
    assert_eq!((pattern.order, pattern.submit_sequence), (1, 1));

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
                && vertex.overlay == pattern.overlay
        ));
}

fn tropical_fish_texture_images() -> Vec<EntityModelTextureImage> {
    tropical_fish_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

#[test]
fn tropical_fish_pattern_from_vanilla_packed_variant() {
    // `TropicalFish.Pattern.byId(packed & 0xFFFF)`, keyed on `packedId = base.id | index << 8`
    // (`SMALL=0`/`LARGE=1`, index `0..=5`); unknown ids fall back to KOB.
    let cases = [
        (0x0000, TropicalFishPattern::Kob),
        (0x0100, TropicalFishPattern::Sunstreak),
        (0x0200, TropicalFishPattern::Snooper),
        (0x0300, TropicalFishPattern::Dasher),
        (0x0400, TropicalFishPattern::Brinely),
        (0x0500, TropicalFishPattern::Spotty),
        (0x0001, TropicalFishPattern::Flopper),
        (0x0101, TropicalFishPattern::Stripey),
        (0x0201, TropicalFishPattern::Glitter),
        (0x0301, TropicalFishPattern::Blockfish),
        (0x0401, TropicalFishPattern::Betty),
        (0x0501, TropicalFishPattern::Clayfish),
    ];
    for (packed, expected) in cases {
        assert_eq!(
            TropicalFishPattern::from_vanilla_packed_variant(packed),
            expected
        );
        // The base color (bits 16..24) and pattern color (bits 24..32) bytes never disturb it.
        let noisy = packed | (0x09 << 16) | (0x0Du32 << 24) as i32;
        assert_eq!(
            TropicalFishPattern::from_vanilla_packed_variant(noisy),
            expected
        );
    }
    // Unknown low-byte base (0xAB) and out-of-range index (6 → 0x0601) both fall back to KOB.
    assert_eq!(
        TropicalFishPattern::from_vanilla_packed_variant(0x00AB),
        TropicalFishPattern::Kob
    );
    assert_eq!(
        TropicalFishPattern::from_vanilla_packed_variant(0x0601),
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

#[test]
fn tropical_fish_pattern_geometry_inflates_base_by_fish_pattern_deformation() {
    // Vanilla `ModelLayers.TROPICAL_FISH_*_PATTERN = createBodyLayer(FISH_PATTERN_DEFORMATION)`:
    // the overlay is the base body grown by 0.008 on every face (`min -= g`, `size += 2·g`),
    // keeping the base box for UVs and the same `texOffs`/mirror.
    assert_eq!(FISH_PATTERN_DEFORMATION, 0.008);
    let g = FISH_PATTERN_DEFORMATION;
    let base_cubes = [
        TROPICAL_FISH_SMALL_BODY[0],
        TROPICAL_FISH_SMALL_TAIL[0],
        TROPICAL_FISH_SMALL_RIGHT_FIN[0],
        TROPICAL_FISH_SMALL_LEFT_FIN[0],
        TROPICAL_FISH_SMALL_TOP_FIN[0],
        TROPICAL_FISH_LARGE_BODY[0],
        TROPICAL_FISH_LARGE_TAIL[0],
        TROPICAL_FISH_LARGE_RIGHT_FIN[0],
        TROPICAL_FISH_LARGE_LEFT_FIN[0],
        TROPICAL_FISH_LARGE_TOP_FIN[0],
        TROPICAL_FISH_LARGE_BOTTOM_FIN[0],
    ];
    let pattern_cubes = [
        TROPICAL_FISH_SMALL_PATTERN_BODY[0],
        TROPICAL_FISH_SMALL_PATTERN_TAIL[0],
        TROPICAL_FISH_SMALL_PATTERN_RIGHT_FIN[0],
        TROPICAL_FISH_SMALL_PATTERN_LEFT_FIN[0],
        TROPICAL_FISH_SMALL_PATTERN_TOP_FIN[0],
        TROPICAL_FISH_LARGE_PATTERN_BODY[0],
        TROPICAL_FISH_LARGE_PATTERN_TAIL[0],
        TROPICAL_FISH_LARGE_PATTERN_RIGHT_FIN[0],
        TROPICAL_FISH_LARGE_PATTERN_LEFT_FIN[0],
        TROPICAL_FISH_LARGE_PATTERN_TOP_FIN[0],
        TROPICAL_FISH_LARGE_PATTERN_BOTTOM_FIN[0],
    ];
    let mut count = 0;
    for (base, pattern) in base_cubes.iter().zip(pattern_cubes.iter()) {
        for axis in 0..3 {
            assert!((pattern.min[axis] - (base.min[axis] - g)).abs() < 1.0e-7);
            assert!((pattern.size[axis] - (base.size[axis] + 2.0 * g)).abs() < 1.0e-7);
        }
        assert_eq!(pattern.uv_size, base.uv_size);
        assert_eq!(pattern.tex, base.tex);
        assert_eq!(pattern.mirror, base.mirror);
        count += 1;
    }
    // Five small body cubes plus six large body cubes are inflated for the overlay.
    assert_eq!(count, 11);
}
