use super::*;

#[test]
fn tropical_fish_small_parts_match_vanilla_26_1_body_layer() {
    // Vanilla `TropicalFishSmallModel.createBodyLayer` (kob body, atlas 32×32): body, tail,
    // right fin (`yRot = π/4`), left fin (`yRot = -π/4`), top fin.
    assert_eq!(TROPICAL_FISH_SMALL_PARTS.len(), 5);

    assert_part(
        &TROPICAL_FISH_SMALL_PARTS[0],
        [0.0, 22.0, 0.0],
        [0.0, 0.0, 0.0],
        TROPICAL_FISH_SMALL_BODY.as_slice(),
    );
    assert_eq!(
        TROPICAL_FISH_SMALL_BODY[0],
        ModelCubeDesc {
            min: [-1.0, -1.5, -3.0],
            size: [2.0, 3.0, 6.0],
            color: TROPICAL_FISH_ORANGE,
        }
    );

    // The tail is index `TROPICAL_FISH_TAIL_PART_INDEX`; it is a zero-thickness plane.
    assert_eq!(TROPICAL_FISH_TAIL_PART_INDEX, 1);
    assert_part(
        &TROPICAL_FISH_SMALL_PARTS[1],
        [0.0, 22.0, 3.0],
        [0.0, 0.0, 0.0],
        TROPICAL_FISH_SMALL_TAIL.as_slice(),
    );
    assert_eq!(TROPICAL_FISH_SMALL_TAIL[0].size, [0.0, 3.0, 6.0]);

    // The side fins splay ±π/4 about Y (not Z like cod/salmon).
    assert_part(
        &TROPICAL_FISH_SMALL_PARTS[2],
        [-1.0, 22.5, 0.0],
        [0.0, std::f32::consts::FRAC_PI_4, 0.0],
        TROPICAL_FISH_SMALL_RIGHT_FIN.as_slice(),
    );
    assert_part(
        &TROPICAL_FISH_SMALL_PARTS[3],
        [1.0, 22.5, 0.0],
        [0.0, -std::f32::consts::FRAC_PI_4, 0.0],
        TROPICAL_FISH_SMALL_LEFT_FIN.as_slice(),
    );
    assert_eq!(TROPICAL_FISH_SMALL_RIGHT_FIN[0].size, [2.0, 2.0, 0.0]);

    assert_part(
        &TROPICAL_FISH_SMALL_PARTS[4],
        [0.0, 20.5, -3.0],
        [0.0, 0.0, 0.0],
        TROPICAL_FISH_SMALL_TOP_FIN.as_slice(),
    );
    assert_eq!(TROPICAL_FISH_SMALL_TOP_FIN[0].size, [0.0, 3.0, 6.0]);
}

#[test]
fn tropical_fish_large_parts_match_vanilla_26_1_body_layer() {
    // Vanilla `TropicalFishLargeModel.createBodyLayer` (flopper body, atlas 32×32): body,
    // tail, right fin, left fin, top fin, bottom fin.
    assert_eq!(TROPICAL_FISH_LARGE_PARTS.len(), 6);

    assert_part(
        &TROPICAL_FISH_LARGE_PARTS[0],
        [0.0, 19.0, 0.0],
        [0.0, 0.0, 0.0],
        TROPICAL_FISH_LARGE_BODY.as_slice(),
    );
    assert_eq!(
        TROPICAL_FISH_LARGE_BODY[0],
        ModelCubeDesc {
            min: [-1.0, -3.0, -3.0],
            size: [2.0, 6.0, 6.0],
            color: TROPICAL_FISH_ORANGE,
        }
    );

    assert_eq!(TROPICAL_FISH_TAIL_PART_INDEX, 1);
    assert_part(
        &TROPICAL_FISH_LARGE_PARTS[1],
        [0.0, 19.0, 3.0],
        [0.0, 0.0, 0.0],
        TROPICAL_FISH_LARGE_TAIL.as_slice(),
    );
    assert_eq!(TROPICAL_FISH_LARGE_TAIL[0].size, [0.0, 6.0, 5.0]);

    assert_part(
        &TROPICAL_FISH_LARGE_PARTS[2],
        [-1.0, 20.0, 0.0],
        [0.0, std::f32::consts::FRAC_PI_4, 0.0],
        TROPICAL_FISH_LARGE_RIGHT_FIN.as_slice(),
    );
    assert_part(
        &TROPICAL_FISH_LARGE_PARTS[3],
        [1.0, 20.0, 0.0],
        [0.0, -std::f32::consts::FRAC_PI_4, 0.0],
        TROPICAL_FISH_LARGE_LEFT_FIN.as_slice(),
    );

    assert_part(
        &TROPICAL_FISH_LARGE_PARTS[4],
        [0.0, 16.0, -3.0],
        [0.0, 0.0, 0.0],
        TROPICAL_FISH_LARGE_TOP_FIN.as_slice(),
    );
    assert_eq!(TROPICAL_FISH_LARGE_TOP_FIN[0].size, [0.0, 4.0, 6.0]);

    // The bottom fin is unique to the large (flopper) body.
    assert_part(
        &TROPICAL_FISH_LARGE_PARTS[5],
        [0.0, 22.0, -3.0],
        [0.0, 0.0, 0.0],
        TROPICAL_FISH_LARGE_BOTTOM_FIN.as_slice(),
    );
    assert_eq!(TROPICAL_FISH_LARGE_BOTTOM_FIN[0].size, [0.0, 4.0, 6.0]);
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
    )
    .with_in_water(true)]);
    assert_eq!(fish.opaque_faces, 30);
    assert_eq!(fish.vertices.len(), 120);
    assert_eq!(fish.indices.len(), 180);
    assert!(fish
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(TROPICAL_FISH_ORANGE, 1.0)));
}

#[test]
fn tropical_fish_large_mesh_uses_vanilla_geometry() {
    // Six cubes → 36 faces / 144 vertices.
    let fish = entity_model_mesh(&[EntityModelInstance::tropical_fish(
        801,
        [0.0, 64.0, 0.0],
        0.0,
        TropicalFishModelShape::Large,
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
fn tropical_fish_texture_ref_matches_vanilla_renderer() {
    // `TropicalFishRenderer` keys the small body on `tropical_a` and the large on
    // `tropical_b`; the model layers are `ModelLayers.TROPICAL_FISH_{SMALL,LARGE}`.
    let small = EntityModelKind::TropicalFish {
        shape: TropicalFishModelShape::Small,
    };
    let large = EntityModelKind::TropicalFish {
        shape: TropicalFishModelShape::Large,
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
