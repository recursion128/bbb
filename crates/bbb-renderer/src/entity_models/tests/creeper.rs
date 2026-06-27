use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn creeper_model_parts_match_vanilla_26_1_body_layer() {
    // The unified cubes carry both render paths' geometry: the colored debug tint and the textured
    // `uv_size`/`texOffs`. Vanilla `CreeperModel.createBodyLayer(CubeDeformation.NONE)`.
    assert_eq!(
        CREEPER_HEAD[0],
        ModelCube::new(
            [-4.0, -8.0, -4.0],
            [8.0, 8.0, 8.0],
            CREEPER_GREEN,
            [8.0, 8.0, 8.0],
            [0.0, 0.0],
            false,
        )
    );
    assert_eq!(
        CREEPER_BODY[0],
        ModelCube::new(
            [-4.0, 0.0, -2.0],
            [8.0, 12.0, 4.0],
            CREEPER_GREEN,
            [8.0, 12.0, 4.0],
            [16.0, 16.0],
            false,
        )
    );
    assert_eq!(
        CREEPER_LEG[0],
        ModelCube::new(
            [-2.0, 0.0, -2.0],
            [4.0, 6.0, 4.0],
            CREEPER_GREEN,
            [4.0, 6.0, 4.0],
            [0.0, 16.0],
            false,
        )
    );
    assert_eq!(CREEPER_HEAD_POSE.offset, [0.0, 6.0, 0.0]);
}

#[test]
fn creeper_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::new(
        50,
        EntityModelKind::Creeper,
        [0.0, 64.0, 0.0],
        0.0,
    )]);

    assert_eq!(mesh.opaque_faces, 36);
    assert_eq!(mesh.vertices.len(), 144);
    assert_eq!(mesh.indices.len(), 216);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.25, 64.001, -0.375]);
    assert_close3(max, [0.25, 65.626, 0.375]);
}

#[test]
fn creeper_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::Creeper.model_key(), "creeper");
    assert_eq!(
        EntityModelKind::Creeper.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/creeper/creeper.png",
            size: [64, 32],
        })
    );
    assert_eq!(
        EntityModelKind::Chicken {
            variant: ChickenModelVariant::Temperate,
            baby: false
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/chicken/chicken_temperate.png",
            size: [64, 32],
        })
    );
}

#[test]
fn creeper_textured_layer_passes_match_vanilla_renderer_model_layer() {
    let passes = creeper_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::CreeperBase);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_CREEPER);
    assert_eq!(passes[0].texture, CREEPER_TEXTURE_REF);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((passes[0].order, passes[0].submit_sequence), (0, 0));
}

#[test]
fn creeper_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    // The textured UV sources now live on the unified cubes (`uv_size`/`tex`/`mirror`).
    assert_eq!(MODEL_LAYER_CREEPER, "minecraft:creeper#main");
    assert_eq!(CREEPER_HEAD[0].uv_size, [8.0, 8.0, 8.0]);
    assert_eq!(CREEPER_HEAD[0].tex, [0.0, 0.0]);
    assert!(!CREEPER_HEAD[0].mirror);
    assert_eq!(CREEPER_BODY[0].uv_size, [8.0, 12.0, 4.0]);
    assert_eq!(CREEPER_BODY[0].tex, [16.0, 16.0]);
    assert!(!CREEPER_BODY[0].mirror);
    assert_eq!(CREEPER_LEG[0].uv_size, [4.0, 6.0, 4.0]);
    assert_eq!(CREEPER_LEG[0].tex, [0.0, 16.0]);
    assert!(!CREEPER_LEG[0].mirror);
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
    assert_eq!(plain.submissions.len(), 1);
    assert_creeper_base_submission_matches_vanilla(
        &plain,
        EntityModelInstance::new(960, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0),
    );
    assert!(!plain
        .submissions
        .iter()
        .any(|submit| submit.render_type == EntityModelLayerRenderType::EnergySwirl));

    // A charged creeper draws the inflated `CREEPER_ARMOR` model (6 cubes → 144 vertices) into the
    // additive scroll mesh, every vertex tinted by the vanilla `0xFF808080` half-grey.
    let grey = 128.0 / 255.0;
    let powered = EntityModelInstance::new(961, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0)
        .with_creeper_powered(true);
    let rest = entity_model_textured_meshes(&[powered], &atlas);
    assert_eq!(rest.submissions.len(), 2);
    assert_creeper_base_submission_matches_vanilla(&rest, powered);
    let swirl = rest
        .submissions
        .iter()
        .find(|submit| submit.render_type == EntityModelLayerRenderType::EnergySwirl)
        .expect("powered creeper emits an energySwirl submit");
    assert_eq!(swirl.render_type.vanilla_name(), "energySwirl");
    assert_eq!(swirl.texture, CREEPER_ARMOR_TEXTURE_REF);
    assert_eq!(swirl.tint, [grey, grey, grey, 1.0]);
    assert_eq!(swirl.order, 1);
    assert_eq!(swirl.submit_sequence, 1);
    assert_eq!(swirl.transform, creeper_model_root_transform(powered));
    assert_eq!(rest.scroll_additive.vertices.len(), 144);
    assert!(rest
        .scroll_additive
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [grey, grey, grey, 1.0]));

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
            EntityModelInstance::new(961, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0)
                .with_creeper_powered(true)
                .with_age_in_ticks(age),
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
fn charged_creeper_energy_swirl_submission_survives_missing_armor_atlas_entry() {
    // `EnergySwirlLayer` is a residual overlay, but it must still produce its vanilla submission before
    // the backend tries to fold the scrolled geometry into the additive mesh bucket.
    let len =
        usize::try_from(CREEPER_TEXTURE_REF.size[0] * CREEPER_TEXTURE_REF.size[1] * 4).unwrap();
    let images = vec![EntityModelTextureImage::new(
        CREEPER_TEXTURE_REF,
        vec![0u8; len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let grey = 128.0 / 255.0;
    let light_coords = (5_u32 << 4) | (11_u32 << 20);
    let powered = EntityModelInstance::new(962, EntityModelKind::Creeper, [1.0, 64.0, 2.0], 45.0)
        .with_creeper_powered(true)
        .with_light_coords(light_coords)
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);

    let meshes = entity_model_textured_meshes(&[powered], &atlas);
    assert_eq!(meshes.submissions.len(), 2);
    assert_creeper_base_submission_matches_vanilla(&meshes, powered);
    assert_eq!(
        meshes.cutout.vertices.len(),
        144,
        "base creeper still renders from the available base texture"
    );
    assert!(
        meshes.scroll_additive.vertices.is_empty(),
        "missing creeper_armor.png suppresses only folded energy-swirl geometry"
    );
    let swirl = meshes
        .submissions
        .iter()
        .find(|submit| submit.render_type == EntityModelLayerRenderType::EnergySwirl)
        .expect("powered creeper records an energySwirl submit before atlas lookup");
    assert_eq!(swirl.render_type.vanilla_name(), "energySwirl");
    assert_eq!(swirl.texture, CREEPER_ARMOR_TEXTURE_REF);
    assert_eq!(swirl.tint, [grey, grey, grey, 1.0]);
    assert_eq!((swirl.order, swirl.submit_sequence), (1, 1));
    assert_eq!(swirl.transform, creeper_model_root_transform(powered));
    assert_eq!(swirl.light, powered.render_state.shader_light());
    assert_eq!(swirl.overlay, [0.0, 10.0]);
    assert_ne!(swirl.overlay, powered.render_state.overlay_coords());
}

#[test]
fn creeper_textured_mesh_uses_vanilla_uvs_tints_and_body_layer_bounds() {
    let (atlas, _) = build_entity_model_texture_atlas(&creeper_texture_images()).unwrap();
    let instance = EntityModelInstance::new(910, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0);
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert_eq!(meshes.submissions.len(), 1);
    assert_creeper_base_submission_matches_vanilla(&meshes, instance);
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
    let (min, max) = textured_mesh_extents(&mesh);
    assert_close3(min, [-0.25, 64.001, -0.375]);
    assert_close3(max, [0.25, 65.626, 0.375]);
}

#[test]
fn creeper_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&creeper_texture_images()).unwrap();
    let base = EntityModelInstance::new(199, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0);
    let yawed_instance = base.with_head_look(45.0, 0.0);
    let pitched_instance = base.with_head_look(0.0, -20.0);
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let yawed = entity_model_textured_meshes(&[yawed_instance], &atlas);
    let pitched = entity_model_textured_meshes(&[pitched_instance], &atlas);
    assert_eq!(resting.submissions.len(), 1);
    assert_eq!(yawed.submissions.len(), 1);
    assert_eq!(pitched.submissions.len(), 1);
    assert_creeper_base_submission_matches_vanilla(&resting, base);
    assert_creeper_base_submission_matches_vanilla(&yawed, yawed_instance);
    assert_creeper_base_submission_matches_vanilla(&pitched, pitched_instance);
    assert!(resting.scroll_additive.vertices.is_empty());
    assert!(yawed.scroll_additive.vertices.is_empty());
    assert!(pitched.scroll_additive.vertices.is_empty());
    assert_eq!(resting.cutout.vertices.len(), yawed.cutout.vertices.len());
    assert_ne!(resting.cutout.vertices, yawed.cutout.vertices);
    assert_ne!(yawed.cutout.vertices, pitched.cutout.vertices);
}

#[test]
fn creeper_swings_its_legs_when_walking() {
    // Vanilla `CreeperModel` is a custom `EntityModel` whose `setupAnim` leg swing is
    // exactly the `QuadrupedModel` formula (`cos(pos * 0.6662 [+ π]) * 1.4 * speed`,
    // hind-right/front-left in phase, legs at [2, 3, 4, 5]). A standing creeper is
    // inert; a walking one lifts its feet and splays its legs along Z (the swell scale
    // is exercised separately below). Colored path here, textured below.
    let base = EntityModelInstance::new(250, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0);
    let rest = entity_model_mesh(&[base]);
    let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
    assert_eq!(rest.vertices, still.vertices, "rest is inert");

    let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
    assert_ne!(rest.vertices, walking.vertices, "walking differs");

    let (rest_min, rest_max) = mesh_extents(&rest);
    let (walk_min, walk_max) = mesh_extents(&walking);
    assert!(
        (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
        "a walking creeper's feet should lift off the ground"
    );
    assert!(
        (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
        "a walking creeper's legs should splay along Z"
    );
}

#[test]
fn creeper_textured_mesh_swings_legs_when_walking() {
    // The real creeper render path (texture-backed) swings the same `QuadrupedModel`
    // legs. A standing creeper is byte-identical however far the swing has advanced;
    // a walking one lifts its feet.
    let (atlas, _) = build_entity_model_texture_atlas(&creeper_texture_images()).unwrap();
    let base = EntityModelInstance::new(251, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0);
    let still_instance = base.with_walk_animation(2.5, 0.0);
    let walking_instance = base.with_walk_animation(0.0, 1.0);
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let still = entity_model_textured_meshes(&[still_instance], &atlas);
    let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
    assert_eq!(resting.submissions.len(), 1);
    assert_eq!(still.submissions.len(), 1);
    assert_eq!(walking.submissions.len(), 1);
    assert_creeper_base_submission_matches_vanilla(&resting, base);
    assert_creeper_base_submission_matches_vanilla(&still, still_instance);
    assert_creeper_base_submission_matches_vanilla(&walking, walking_instance);
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
fn creeper_inflates_as_it_primes_to_explode() {
    // Vanilla `CreeperRenderer.scale` applies a non-uniform swell scale at the `this.scale()`
    // hook while a creeper primes to explode:
    //   wobble = 1 + sin(swelling * 100) * swelling * 0.01
    //   g = clamp(swelling, 0, 1)^4
    //   x/z *= (1 + g * 0.4) * wobble,   y *= (1 + g * 0.1) / wobble
    // At swelling 0 it is the identity, so a calm creeper is byte-identical. The model scales
    // about its local root (and the default 180-degree body yaw only flips X/Z), so each axis'
    // extent grows by exactly its factor. Colored path here, textured below.
    let base = EntityModelInstance::new(252, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0);
    let calm = entity_model_mesh(&[base]);
    let unswollen = entity_model_mesh(&[base.with_creeper_swelling(0.0)]);
    assert_eq!(
        calm.vertices, unswollen.vertices,
        "an unswollen creeper is unscaled"
    );

    let swelling = 1.0_f32;
    let primed = entity_model_mesh(&[base.with_creeper_swelling(swelling)]);
    assert_ne!(
        calm.vertices, primed.vertices,
        "a priming creeper is rescaled"
    );

    let wobble = 1.0 + (swelling * 100.0).sin() * swelling * 0.01;
    let g = swelling.clamp(0.0, 1.0);
    let g = g * g * g * g;
    let expected_s = (1.0 + g * 0.4) * wobble;
    let expected_hs = (1.0 + g * 0.1) / wobble;
    assert!(
        expected_s > 1.0 && expected_hs > 1.0,
        "a fully primed creeper grows on every axis"
    );

    let (calm_min, calm_max) = mesh_extents(&calm);
    let (primed_min, primed_max) = mesh_extents(&primed);
    let assert_ratio = |primed_w: f32, calm_w: f32, expected: f32| {
        assert!(
            (primed_w / calm_w - expected).abs() < 1.0e-4,
            "extent ratio {} should match swell factor {expected}",
            primed_w / calm_w
        );
    };
    assert_ratio(
        primed_max[0] - primed_min[0],
        calm_max[0] - calm_min[0],
        expected_s,
    );
    assert_ratio(
        primed_max[2] - primed_min[2],
        calm_max[2] - calm_min[2],
        expected_s,
    );
    assert_ratio(
        primed_max[1] - primed_min[1],
        calm_max[1] - calm_min[1],
        expected_hs,
    );
}

#[test]
fn creeper_textured_mesh_inflates_as_it_primes_to_explode() {
    // The real creeper render path (texture-backed) inflates with the same
    // `CreeperRenderer.scale` swell. An unswollen creeper is byte-identical; a fully primed
    // one grows on the X/Z plane by `(1 + 0.4) * wobble`.
    let (atlas, _) = build_entity_model_texture_atlas(&creeper_texture_images()).unwrap();
    let base = EntityModelInstance::new(253, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0);
    let unswollen_instance = base.with_creeper_swelling(0.0);
    let calm = entity_model_textured_meshes(&[base], &atlas);
    let unswollen = entity_model_textured_meshes(&[unswollen_instance], &atlas);
    assert_eq!(calm.submissions.len(), 1);
    assert_eq!(unswollen.submissions.len(), 1);
    assert_creeper_base_submission_matches_vanilla(&calm, base);
    assert_creeper_base_submission_matches_vanilla(&unswollen, unswollen_instance);
    assert!(calm.scroll_additive.vertices.is_empty());
    assert!(unswollen.scroll_additive.vertices.is_empty());
    assert_eq!(
        calm.cutout.vertices, unswollen.cutout.vertices,
        "an unswollen creeper is unscaled"
    );

    let swelling = 1.0_f32;
    let primed_instance = base.with_creeper_swelling(swelling);
    let primed = entity_model_textured_meshes(&[primed_instance], &atlas);
    assert_eq!(primed.submissions.len(), 1);
    assert_creeper_base_submission_matches_vanilla(&primed, primed_instance);
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

fn assert_creeper_base_submission_matches_vanilla(
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

    let submit = meshes.submissions[0];
    assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(submit.texture, CREEPER_TEXTURE_REF);
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(submit.transform, creeper_model_root_transform(instance));
    assert_eq!(submit.light, instance.render_state.shader_light());
    assert_eq!(submit.overlay, instance.render_state.overlay_coords());
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));
}

fn creeper_texture_images() -> Vec<EntityModelTextureImage> {
    creeper_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
