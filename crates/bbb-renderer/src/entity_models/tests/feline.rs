use super::*;
use crate::entity_models::model::EntityModel;

#[test]
fn feline_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `AdultFelineModel.createBodyMesh(NONE)` (atlas 64×32): eight flat named root parts —
    // head, body, two tail segments, and four legs.

    // `head` (offset (0, 15, -9)): the 5×4×5 skull, the 3×2×2 nose, and the two 1×1×2 ears.
    assert_eq!(FELINE_HEAD_POSE.offset, [0.0, 15.0, -9.0]);
    assert_eq!(FELINE_HEAD_CUBES.len(), 4);
    assert_eq!(FELINE_HEAD_CUBES[0].min, [-2.5, -2.0, -3.0]);
    assert_eq!(FELINE_HEAD_CUBES[0].size, [5.0, 4.0, 5.0]);
    assert_eq!(FELINE_HEAD_CUBES[1].min, [-1.5, -0.001, -4.0]);
    assert_eq!(FELINE_HEAD_CUBES[1].size, [3.0, 2.0, 2.0]);
    assert_eq!(FELINE_HEAD_CUBES[2].min, [-2.0, -3.0, 0.0]);
    assert_eq!(FELINE_HEAD_CUBES[3].min, [1.0, -3.0, 0.0]);

    // `body` (offset (0, 12, -10), pitched π/2): the 4×16×6 trunk.
    assert_eq!(FELINE_BODY_POSE.offset, [0.0, 12.0, -10.0]);
    assert_eq!(
        FELINE_BODY_POSE.rotation,
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0]
    );
    assert_eq!(FELINE_BODY_CUBES[0].min, [-2.0, 3.0, -8.0]);
    assert_eq!(FELINE_BODY_CUBES[0].size, [4.0, 16.0, 6.0]);

    // `tail1` (offset (0, 15, 8), pitched 0.9): the upper 1×8×1 segment.
    assert_eq!(FELINE_TAIL1_POSE.offset, [0.0, 15.0, 8.0]);
    assert_eq!(FELINE_TAIL1_POSE.rotation, [0.9, 0.0, 0.0]);
    assert_eq!(FELINE_TAIL1_CUBES[0].size, [1.0, 8.0, 1.0]);

    // `tail2` (offset (0, 20, 14)): the lower segment, deflated by the vanilla `CubeDeformation(-0.02)`.
    assert_eq!(FELINE_TAIL2_POSE.offset, [0.0, 20.0, 14.0]);
    assert_eq!(FELINE_TAIL2_POSE.rotation, [0.0, 0.0, 0.0]);
    assert_eq!(FELINE_TAIL2_CUBES[0].min, [-0.48, 0.02, 0.02]);
    assert_eq!(FELINE_TAIL2_CUBES[0].size, [0.96, 7.96, 0.96]);

    // The four legs: hind (2×6×2) at z=5, front (2×10×2) at z=-5, mirrored on X.
    assert_eq!(FELINE_LEFT_HIND_LEG_POSE.offset, [1.1, 18.0, 5.0]);
    assert_eq!(FELINE_RIGHT_HIND_LEG_POSE.offset, [-1.1, 18.0, 5.0]);
    assert_eq!(FELINE_HIND_LEG_CUBES[0].size, [2.0, 6.0, 2.0]);
    assert_eq!(FELINE_LEFT_FRONT_LEG_POSE.offset, [1.2, 14.1, -5.0]);
    assert_eq!(FELINE_RIGHT_FRONT_LEG_POSE.offset, [-1.2, 14.1, -5.0]);
    assert_eq!(FELINE_FRONT_LEG_CUBES[0].size, [2.0, 10.0, 2.0]);
}

#[test]
fn baby_feline_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `BabyFelineModel.createBodyMesh` (atlas 32×32): eight flat named root parts in a
    // flatter, all-upright layout — head, three legs, body, the fourth leg, then the two tail segments.

    // `head` (offset (0, 20, -3.125)): the 5×4×4 skull, two 1×1×2 ears, and a 3×2×1 nose.
    assert_eq!(BABY_FELINE_HEAD_POSE.offset, [0.0, 20.0, -3.125]);
    assert_eq!(BABY_FELINE_HEAD_CUBES.len(), 4);
    assert_eq!(BABY_FELINE_HEAD_CUBES[0].min, [-2.5, -3.0, -2.875]);
    assert_eq!(BABY_FELINE_HEAD_CUBES[0].size, [5.0, 4.0, 4.0]);
    assert_eq!(BABY_FELINE_HEAD_CUBES[1].min, [-2.0, -4.0, -0.875]);
    assert_eq!(BABY_FELINE_HEAD_CUBES[2].min, [1.0, -4.0, -0.875]);
    assert_eq!(BABY_FELINE_HEAD_CUBES[3].min, [-1.5, -1.0, -3.875]);
    assert_eq!(BABY_FELINE_HEAD_CUBES[3].size, [3.0, 2.0, 1.0]);

    // The four 1×2×2 legs (same box dims, but each leg carries its own texOffs): left/right front at
    // z=-1.5, left/right hind at z=2.5.
    assert_eq!(BABY_FELINE_LEFT_FRONT_LEG_POSE.offset, [1.0, 22.0, -1.5]);
    assert_eq!(BABY_FELINE_RIGHT_FRONT_LEG_POSE.offset, [-1.0, 22.0, -1.5]);
    assert_eq!(BABY_FELINE_LEFT_HIND_LEG_POSE.offset, [1.0, 22.0, 2.5]);
    assert_eq!(BABY_FELINE_RIGHT_HIND_LEG_POSE.offset, [-1.0, 22.0, 2.5]);
    assert_eq!(BABY_FELINE_LEFT_FRONT_LEG_CUBES[0].min, [-0.5, 0.0, -1.0]);
    assert_eq!(BABY_FELINE_LEFT_FRONT_LEG_CUBES[0].size, [1.0, 2.0, 2.0]);

    // `body` (offset (0, 20.5, 0.5)): the 4×3×7 trunk, upright (no pitch).
    assert_eq!(BABY_FELINE_BODY_POSE.offset, [0.0, 20.5, 0.5]);
    assert_eq!(BABY_FELINE_BODY_POSE.rotation, [0.0, 0.0, 0.0]);
    assert_eq!(BABY_FELINE_BODY_CUBES[0].min, [-2.0, -1.5, -3.5]);
    assert_eq!(BABY_FELINE_BODY_CUBES[0].size, [4.0, 3.0, 7.0]);

    // `tail1` (offset (0, 19.107, 3.9151), pitched -0.567232): the single 1×1×5 segment.
    assert_eq!(BABY_FELINE_TAIL1_POSE.offset, [0.0, 19.107, 3.9151]);
    assert_eq!(BABY_FELINE_TAIL1_POSE.rotation, [-0.567232, 0.0, 0.0]);
    assert_eq!(BABY_FELINE_TAIL1_CUBES[0].size, [1.0, 1.0, 5.0]);

    // `tail2` (PartPose.ZERO): a cubeless pivot.
    assert_eq!(BABY_FELINE_TAIL2_POSE.offset, [0.0, 0.0, 0.0]);
}

#[test]
fn feline_mesh_uses_vanilla_body_layer_geometry() {
    // 11 cubes → 66 faces / 264 vertices / 396 indices, one tan tint.
    let ocelot = entity_model_mesh(&[EntityModelInstance::feline(
        500,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        CatModelVariant::Black,
    )]);
    assert_eq!(ocelot.opaque_faces, 66);
    assert_eq!(ocelot.vertices.len(), 264);
    assert_eq!(ocelot.indices.len(), 396);
    assert!(ocelot
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(FELINE_TAN, 1.0)));
}

#[test]
fn baby_feline_mesh_uses_vanilla_body_layer_geometry() {
    // 10 cubes → 60 faces / 240 vertices / 360 indices, one tan tint.
    let baby = entity_model_mesh(&[EntityModelInstance::feline(
        506,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        CatModelVariant::Black,
    )]);
    assert_eq!(baby.opaque_faces, 60);
    assert_eq!(baby.vertices.len(), 240);
    assert_eq!(baby.indices.len(), 360);
    assert!(baby
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(FELINE_TAN, 1.0)));
}

#[test]
fn feline_colored_runtime_skips_the_texture_backed_feline() {
    // The cat and ocelot now carry vanilla texture UVs, so they render through the textured path. The
    // texture-skipping colored runtime path emits nothing for them (any cat/ocelot × age combo), while
    // the full path still emits the colored fallback geometry.
    let instances = [
        EntityModelInstance::feline(
            501,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            CatModelVariant::Black,
        ),
        EntityModelInstance::feline(
            507,
            [4.0, 64.0, 0.0],
            0.0,
            true,
            true,
            CatModelVariant::Black,
        ),
    ];
    assert!(!entity_model_mesh(&instances).vertices.is_empty());
    assert!(entity_model_colored_runtime_mesh(&instances)
        .vertices
        .is_empty());
}

#[test]
fn baby_cat_and_baby_ocelot_share_the_same_mesh() {
    // Vanilla `CAT_BABY` and `OCELOT_BABY` both use the unscaled `felineBabyBodyLayer`, so the two
    // babies render the identical mesh regardless of the `cat` flag.
    let cat = entity_model_mesh(&[EntityModelInstance::feline(
        508,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        true,
        CatModelVariant::Black,
    )]);
    let ocelot = entity_model_mesh(&[EntityModelInstance::feline(
        508,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        CatModelVariant::Black,
    )]);
    assert_eq!(cat.vertices, ocelot.vertices);
}

#[test]
fn feline_head_look_turns_only_the_head() {
    // Vanilla `AdultFelineModel.setupAnim` sets `head.xRot/yRot` from the look angles. The head is the
    // first root part (four cubes → vertices `[0, 96)`); the body, tail, and legs `[96, 264)` hold (the
    // standing tail droop is applied identically at both, so it does not differ).
    let rest = EntityModelInstance::feline(
        502,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        CatModelVariant::Black,
    );
    let looked = rest.with_head_look(35.0, -25.0);
    let rest_mesh = entity_model_mesh(&[rest]);
    let looked_mesh = entity_model_mesh(&[looked]);
    assert_eq!(rest_mesh.vertices.len(), looked_mesh.vertices.len());
    assert_ne!(
        rest_mesh.vertices[..96],
        looked_mesh.vertices[..96],
        "the head turns"
    );
    assert_eq!(
        rest_mesh.vertices[96..],
        looked_mesh.vertices[96..],
        "the body, tail, and legs stay put"
    );
}

#[test]
fn feline_legs_swing_with_the_gait() {
    // Vanilla `AdultFelineModel.setupAnim` sweeps the four legs by `cos(pos·0.6662 [+π])·1.0·speed`
    // (the mirror of the QuadrupedModel diagonal: left-hind/right-front in phase). The head (vertices
    // `[0, 96)`) is untouched by the swing; the legs move.
    let rest = EntityModelInstance::feline(
        520,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        CatModelVariant::Black,
    );
    let walking = rest.with_walk_animation(3.0, 0.8);
    let rest_mesh = entity_model_mesh(&[rest]);
    let walk_mesh = entity_model_mesh(&[walking]);
    assert_eq!(rest_mesh.vertices.len(), walk_mesh.vertices.len());
    assert_ne!(
        rest_mesh.vertices, walk_mesh.vertices,
        "the legs swing with the gait"
    );
    assert_eq!(
        rest_mesh.vertices[..96],
        walk_mesh.vertices[..96],
        "the head is untouched by the leg swing"
    );

    // The swing advances with the walk position, and is a no-op at zero speed.
    let walk_later = entity_model_mesh(&[rest.with_walk_animation(6.0, 0.8)]);
    assert_ne!(walk_mesh.vertices, walk_later.vertices);
    let still = entity_model_mesh(&[rest.with_walk_animation(3.0, 0.0)]);
    assert_eq!(still.vertices, rest_mesh.vertices);
}

#[test]
fn baby_feline_head_look_turns_only_the_head() {
    // The baby head is also the first root part (four cubes → vertices `[0, 96)`). Everything below it
    // holds — the baby's `tail2` droop is a no-op (it is cubeless), so the rest of the mesh is rigid.
    let rest = EntityModelInstance::feline(
        509,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        CatModelVariant::Black,
    );
    let looked = rest.with_head_look(35.0, -25.0);
    let rest_mesh = entity_model_mesh(&[rest]);
    let looked_mesh = entity_model_mesh(&[looked]);
    assert_ne!(
        rest_mesh.vertices[..96],
        looked_mesh.vertices[..96],
        "the head turns"
    );
    assert_eq!(
        rest_mesh.vertices[96..],
        looked_mesh.vertices[96..],
        "the legs, body, and tail stay put"
    );
}

#[test]
fn feline_standing_drops_the_lower_tail() {
    // Vanilla `AdultFelineModel.setupAnim` sets `tail2.xRot = 1.7278761` while not sitting (the base the
    // deferred walk wobble adds onto), a real change from the `0` bind rotation; the bind-0.9 `tail1`
    // is left alone at rest.
    let mut model = FelineModel::new(false);
    model.prepare(&EntityModelInstance::feline(
        503,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        CatModelVariant::Black,
    ));
    assert!((model.root_mut().child_mut("tail2").pose.rotation[0] - 1.7278761).abs() < 1.0e-6);
    assert_eq!(model.root_mut().child_mut("tail1").pose.rotation[0], 0.9);
}

#[test]
fn feline_lower_tail_wobbles_with_the_gait() {
    // Vanilla `AdultFelineModel.setupAnim` (not-sitting, not-sprinting, not-crouching) sets
    // `tail2.xRot = 1.7278761 + (π/4)·cos(walkAnimationPos)·walkAnimationSpeed`. At rest the wobble
    // term is zero, so `tail2.xRot` holds at the standing droop; walking shifts it off the droop.
    let mut model = FelineModel::new(false);

    // At rest (zero speed) the wobble collapses to the bare droop.
    model.prepare(&EntityModelInstance::feline(
        530,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        CatModelVariant::Black,
    ));
    let rest_tail2 = model.root_mut().child_mut("tail2").pose.rotation[0];
    assert!((rest_tail2 - 1.7278761).abs() < 1.0e-6);

    // Walking adds `(π/4)·cos(pos)·speed` onto the droop.
    let pos = 3.0_f32;
    let speed = 0.8_f32;
    model.prepare(
        &EntityModelInstance::feline(
            531,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            CatModelVariant::Black,
        )
        .with_walk_animation(pos, speed),
    );
    let walk_tail2 = model.root_mut().child_mut("tail2").pose.rotation[0];
    let expected = 1.7278761 + std::f32::consts::FRAC_PI_4 * pos.cos() * speed;
    assert!((walk_tail2 - expected).abs() < 1.0e-6);
    assert_ne!(
        rest_tail2, walk_tail2,
        "the lower tail wobbles while moving"
    );

    // The wobble advances with the walk position.
    model.prepare(
        &EntityModelInstance::feline(
            532,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            CatModelVariant::Black,
        )
        .with_walk_animation(6.0, speed),
    );
    let walk_later_tail2 = model.root_mut().child_mut("tail2").pose.rotation[0];
    assert_ne!(walk_tail2, walk_later_tail2);

    // Zero speed at a nonzero position is still a no-op (back at the droop).
    model.prepare(
        &EntityModelInstance::feline(
            533,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            CatModelVariant::Black,
        )
        .with_walk_animation(3.0, 0.0),
    );
    let still_tail2 = model.root_mut().child_mut("tail2").pose.rotation[0];
    assert!((still_tail2 - 1.7278761).abs() < 1.0e-6);
}

#[test]
fn cat_mesh_is_the_ocelot_mesh_scaled_down() {
    // Vanilla `AdultCatModel.CAT_TRANSFORMER = MeshTransformer.scaling(0.8)`: the cat is the same mesh
    // as the ocelot, scaled 0.8 (so the same vertex count but a more compact mesh).
    let ocelot = entity_model_mesh(&[EntityModelInstance::feline(
        504,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        CatModelVariant::Black,
    )]);
    let cat = entity_model_mesh(&[EntityModelInstance::feline(
        505,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        false,
        CatModelVariant::Black,
    )]);
    assert_eq!(ocelot.vertices.len(), cat.vertices.len());
    let (ocelot_min, ocelot_max) = mesh_extents(&ocelot);
    let (cat_min, cat_max) = mesh_extents(&cat);
    let ocelot_span = ocelot_max[1] - ocelot_min[1];
    let cat_span = cat_max[1] - cat_min[1];
    assert!(
        cat_span < ocelot_span,
        "cat y-span {cat_span} should be smaller than ocelot {ocelot_span}"
    );
}

#[test]
fn baby_feline_is_not_scaled_like_the_adult_cat() {
    // Unlike the adult cat, the baby cat does not get the 0.8 `CAT_TRANSFORMER`: the baby cat and baby
    // ocelot share the unscaled `felineBabyBodyLayer`, so their meshes are identical despite the flag.
    let baby_cat = entity_model_mesh(&[EntityModelInstance::feline(
        510,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        true,
        CatModelVariant::Black,
    )]);
    let baby_ocelot = entity_model_mesh(&[EntityModelInstance::feline(
        510,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        CatModelVariant::Black,
    )]);
    let (cat_min, cat_max) = mesh_extents(&baby_cat);
    let (ocelot_min, ocelot_max) = mesh_extents(&baby_ocelot);
    assert_eq!(cat_min, ocelot_min);
    assert_eq!(cat_max, ocelot_max);
}

#[test]
fn feline_exposes_stable_model_keys() {
    // The model key tracks only cat-vs-ocelot and age; the cat breed shares geometry.
    for cat_variant in [CatModelVariant::Tabby, CatModelVariant::Jellie] {
        for (cat, baby, key) in [
            (true, false, "feline_cat"),
            (false, false, "feline_ocelot"),
            (true, true, "feline_cat_baby"),
            (false, true, "feline_ocelot_baby"),
        ] {
            assert_eq!(
                EntityModelKind::Feline {
                    cat,
                    baby,
                    cat_variant
                }
                .model_key(),
                key
            );
        }
    }
}

#[test]
fn feline_textured_render_matches_vanilla_renderer() {
    // The cat and ocelot share one model but bind different images: the per-breed `CatVariant`
    // texture for cats (× age), the `ocelot` texture for ocelots (the breed is then ignored).
    let cat_variants = [
        CatModelVariant::Tabby,
        CatModelVariant::Black,
        CatModelVariant::Red,
        CatModelVariant::Siamese,
        CatModelVariant::BritishShorthair,
        CatModelVariant::Calico,
        CatModelVariant::Persian,
        CatModelVariant::Ragdoll,
        CatModelVariant::White,
        CatModelVariant::Jellie,
        CatModelVariant::AllBlack,
    ];
    for cat_variant in cat_variants {
        for baby in [false, true] {
            // The cat binds its per-breed texture; the ocelot ignores the breed.
            let cat_texture = feline_texture_ref(true, baby, cat_variant);
            assert_eq!(
                feline_textured_layer_passes(true, baby, cat_variant)[0].texture,
                cat_texture
            );
            assert_eq!(
                feline_textured_layer_passes(true, baby, cat_variant)[0].render_type,
                EntityModelLayerRenderType::Cutout
            );
            assert_eq!(
                EntityModelKind::Feline {
                    cat: true,
                    baby,
                    cat_variant
                }
                .vanilla_texture_ref(),
                Some(cat_texture)
            );
            assert!(entity_model_texture_refs().contains(&cat_texture));

            let ocelot_texture = feline_texture_ref(false, baby, cat_variant);
            assert_eq!(
                ocelot_texture,
                if baby {
                    FELINE_OCELOT_BABY_TEXTURE_REF
                } else {
                    FELINE_OCELOT_TEXTURE_REF
                }
            );
            assert_eq!(
                feline_textured_layer_passes(false, baby, cat_variant)[0].texture,
                ocelot_texture
            );
        }
    }
    assert_eq!(feline_entity_texture_refs().len(), 24);

    let images: Vec<EntityModelTextureImage> = feline_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    for (cat, baby) in [(true, false), (true, true), (false, false), (false, true)] {
        let mesh = entity_model_textured_mesh(
            &[EntityModelInstance::feline(
                900,
                [0.0, 64.0, 0.0],
                0.0,
                cat,
                baby,
                CatModelVariant::Black,
            )],
            &atlas,
        );
        assert!(
            !mesh.vertices.is_empty(),
            "cat={cat} baby={baby} emits textured geometry"
        );
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    }
}
