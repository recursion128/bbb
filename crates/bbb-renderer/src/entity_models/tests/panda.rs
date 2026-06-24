use super::*;

#[test]
fn panda_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `PandaModel.createBodyLayer` (atlas 64×64): the head (carrying the skull, muzzle, and two
    // ears), the pitched body, and four legs sharing one box — nine cubes, built into the named tree.

    // `head` (offset (0, 11.5, -17)): the 13×10×9 skull, the 7×5×2 muzzle, and the two 5×4×1 ears.
    assert_eq!(PANDA_HEAD_POSE.offset, [0.0, 11.5, -17.0]);
    assert_eq!(PANDA_HEAD_POSE.rotation, [0.0, 0.0, 0.0]);
    assert_eq!(PANDA_HEAD_CUBES.len(), 4);
    assert_eq!(PANDA_HEAD_CUBES[0].min, [-6.5, -5.0, -4.0]);
    assert_eq!(PANDA_HEAD_CUBES[0].size, [13.0, 10.0, 9.0]);
    assert_eq!(PANDA_HEAD_CUBES[1].min, [-3.5, 0.0, -6.0]);
    assert_eq!(PANDA_HEAD_CUBES[1].size, [7.0, 5.0, 2.0]);
    assert_eq!(PANDA_HEAD_CUBES[2].min, [3.5, -8.0, -1.0]);
    assert_eq!(PANDA_HEAD_CUBES[3].min, [-8.5, -8.0, -1.0]);
    assert_eq!(PANDA_HEAD_CUBES[2].size, [5.0, 4.0, 1.0]);

    // `body` (offset (0, 10, 0), pitched π/2): the 19×26×13 trunk.
    assert_eq!(PANDA_BODY_POSE.offset, [0.0, 10.0, 0.0]);
    assert_eq!(
        PANDA_BODY_POSE.rotation,
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0]
    );
    assert_eq!(PANDA_BODY_CUBES[0].min, [-9.5, -13.0, -6.5]);
    assert_eq!(PANDA_BODY_CUBES[0].size, [19.0, 26.0, 13.0]);

    // The four legs share one 6×9×6 box.
    assert_eq!(PANDA_LEG_CUBES[0].min, [-3.0, 0.0, -3.0]);
    assert_eq!(PANDA_LEG_CUBES[0].size, [6.0, 9.0, 6.0]);
}

#[test]
fn panda_mesh_uses_vanilla_body_layer_geometry() {
    // 9 cubes → 54 faces / 216 vertices / 324 indices, two tones: white body/head/muzzle, black
    // ears/legs (the per-face directional shading varies the brightness).
    let panda = entity_model_mesh(&[EntityModelInstance::panda(
        600,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        PandaModelVariant::Normal,
    )]);
    assert_eq!(panda.opaque_faces, 54);
    assert_eq!(panda.vertices.len(), 216);
    assert_eq!(panda.indices.len(), 324);
    assert!(panda
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(PANDA_WHITE, 1.0)));
    assert!(panda
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(PANDA_BLACK, 1.0)));
}

#[test]
fn panda_colored_runtime_skips_the_texture_backed_panda() {
    // The panda now carries vanilla texture UVs, so it renders through the textured path. The
    // texture-skipping colored runtime path emits nothing for it, while the full path still emits the
    // colored fallback geometry.
    let instances = [EntityModelInstance::panda(
        601,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        PandaModelVariant::Normal,
    )];
    assert!(!entity_model_mesh(&instances).vertices.is_empty());
    assert!(entity_model_colored_runtime_mesh(&instances)
        .vertices
        .is_empty());
}

#[test]
fn panda_head_look_turns_only_the_head() {
    // Vanilla `QuadrupedModel.setupAnim` sets `head.xRot/yRot` from the look angles. The head is the
    // first root part (four cubes → vertices `[0, 96)`); the body and four legs `[96, 216)` hold.
    let rest =
        EntityModelInstance::panda(602, [0.0, 64.0, 0.0], 0.0, false, PandaModelVariant::Normal);
    let looked = rest.with_head_look(35.0, -25.0);
    let rest_mesh = entity_model_mesh(&[rest]);
    let looked_mesh = entity_model_mesh(&[looked]);
    assert_eq!(rest_mesh.vertices.len(), looked_mesh.vertices.len());
    assert_ne!(
        rest_mesh.vertices[..96],
        looked_mesh.vertices[..96],
        "the head (skull, muzzle, and ears) turns"
    );
    assert_eq!(
        rest_mesh.vertices[96..],
        looked_mesh.vertices[96..],
        "the body and legs stay put"
    );

    // Both yaw and pitch move the head.
    let yaw_only = entity_model_mesh(&[rest.with_head_look(35.0, 0.0)]);
    let pitch_only = entity_model_mesh(&[rest.with_head_look(0.0, -25.0)]);
    assert_ne!(rest_mesh.vertices[..96], yaw_only.vertices[..96]);
    assert_ne!(rest_mesh.vertices[..96], pitch_only.vertices[..96]);
}

#[test]
fn panda_walk_swings_only_the_legs() {
    // Vanilla `QuadrupedModel.setupAnim` swings the four legs off the walk cycle (a no-op at rest).
    // The legs are the last four root parts (vertices `[120, 216)`); the head and body `[0, 120)` hold.
    let still =
        EntityModelInstance::panda(603, [0.0, 64.0, 0.0], 0.0, false, PandaModelVariant::Normal);
    let walking = still.with_walk_animation(6.0, 1.0);
    let still_mesh = entity_model_mesh(&[still]);
    let walking_mesh = entity_model_mesh(&[walking]);
    assert_eq!(still_mesh.vertices.len(), walking_mesh.vertices.len());
    assert_eq!(
        still_mesh.vertices[..120],
        walking_mesh.vertices[..120],
        "the head and body stay put while walking"
    );
    assert_ne!(
        still_mesh.vertices[120..],
        walking_mesh.vertices[120..],
        "the four legs swing off the walk cycle"
    );

    // A standing panda (walk speed 0) collapses the swing to the bind pose.
    let zero_speed = entity_model_mesh(&[EntityModelInstance::panda(
        604,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        PandaModelVariant::Normal,
    )
    .with_walk_animation(6.0, 0.0)]);
    assert_eq!(still_mesh.vertices, zero_speed.vertices);
}

#[test]
fn baby_panda_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `BabyPandaModel.createBodyLayer` (atlas 64×64): the `QuadrupedModel` baby convention lists
    // the body FIRST then the head, and the baby body carries no π/2 pitch.

    // `body` (no pitch): the 9×7×11 trunk.
    assert_eq!(BABY_PANDA_BODY_POSE.offset, [0.0, 18.5, 2.5]);
    assert_eq!(BABY_PANDA_BODY_POSE.rotation, [0.0, 0.0, 0.0]);
    assert_eq!(BABY_PANDA_BODY_CUBES[0].size, [9.0, 7.0, 11.0]);

    // `head`: the 7×6×5 skull, the 4×2×1 muzzle, and the two 3×3×1 ears.
    assert_eq!(BABY_PANDA_HEAD_POSE.offset, [0.0, 19.0, -3.0]);
    assert_eq!(BABY_PANDA_HEAD_CUBES.len(), 4);
    assert_eq!(BABY_PANDA_HEAD_CUBES[0].size, [7.0, 6.0, 5.0]);
    assert_eq!(BABY_PANDA_HEAD_CUBES[2].min, [-4.5, -4.0, -3.5]);
    assert_eq!(BABY_PANDA_HEAD_CUBES[3].min, [1.5, -4.0, -3.5]);

    // The four legs share the 3×2×3 box (one cube const per leg, distinct UV).
    assert_eq!(BABY_PANDA_RIGHT_HIND_LEG_CUBES[0].size, [3.0, 2.0, 3.0]);
}

#[test]
fn baby_panda_head_is_part_one_and_turns_with_the_look() {
    // The baby layout lists the body first (vertices `[0, 24)`) then the head (four cubes,
    // `[24, 120)`), then the four legs. The head look turns the head; the body and legs hold.
    let rest =
        EntityModelInstance::panda(610, [0.0, 64.0, 0.0], 0.0, true, PandaModelVariant::Normal);
    let baby = entity_model_mesh(&[rest]);
    assert_eq!(baby.vertices.len(), 216);
    let looked = entity_model_mesh(&[rest.with_head_look(35.0, -25.0)]);
    assert_eq!(baby.vertices[..24], looked.vertices[..24], "the body holds");
    assert_ne!(
        baby.vertices[24..120],
        looked.vertices[24..120],
        "the head turns"
    );
    assert_eq!(
        baby.vertices[120..],
        looked.vertices[120..],
        "the legs hold at rest"
    );

    // The baby is more compact than the adult (smaller body layer).
    let adult = entity_model_mesh(&[EntityModelInstance::panda(
        611,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        PandaModelVariant::Normal,
    )]);
    let (adult_min, adult_max) = mesh_extents(&adult);
    let (baby_min, baby_max) = mesh_extents(&baby);
    assert!((baby_max[1] - baby_min[1]) < (adult_max[1] - adult_min[1]));
}

#[test]
fn panda_exposes_stable_model_keys() {
    assert_eq!(
        EntityModelKind::Panda {
            baby: false,
            variant: PandaModelVariant::Normal
        }
        .model_key(),
        "panda"
    );
    assert_eq!(
        EntityModelKind::Panda {
            baby: true,
            variant: PandaModelVariant::Aggressive
        }
        .model_key(),
        "panda_baby"
    );
}

#[test]
fn panda_gene_variant_combination_matches_vanilla() {
    // Vanilla `Panda.Gene.byId` is `ByIdMap.continuous` with `OutOfBoundsStrategy.ZERO`.
    assert_eq!(PandaModelVariant::from_id(0), PandaModelVariant::Normal);
    assert_eq!(PandaModelVariant::from_id(1), PandaModelVariant::Lazy);
    assert_eq!(PandaModelVariant::from_id(2), PandaModelVariant::Worried);
    assert_eq!(PandaModelVariant::from_id(3), PandaModelVariant::Playful);
    assert_eq!(PandaModelVariant::from_id(4), PandaModelVariant::Brown);
    assert_eq!(PandaModelVariant::from_id(5), PandaModelVariant::Weak);
    assert_eq!(PandaModelVariant::from_id(6), PandaModelVariant::Aggressive);
    assert_eq!(PandaModelVariant::from_id(7), PandaModelVariant::Normal);
    assert_eq!(PandaModelVariant::from_id(-1), PandaModelVariant::Normal);

    // Vanilla `Panda.Gene.getVariantFromGenes`: a dominant main gene always shows.
    assert_eq!(
        PandaModelVariant::from_genes(1, 4),
        PandaModelVariant::Lazy,
        "dominant LAZY shows regardless of the hidden gene"
    );
    // A recessive main gene (BROWN=4, WEAK=5) shows only when both genes match, else NORMAL.
    assert_eq!(
        PandaModelVariant::from_genes(4, 4),
        PandaModelVariant::Brown,
        "matching recessive BROWN shows"
    );
    assert_eq!(
        PandaModelVariant::from_genes(4, 1),
        PandaModelVariant::Normal,
        "an unmatched recessive BROWN main falls back to NORMAL"
    );
    assert_eq!(
        PandaModelVariant::from_genes(5, 5),
        PandaModelVariant::Weak,
        "matching recessive WEAK shows"
    );
    assert_eq!(
        PandaModelVariant::from_genes(5, 0),
        PandaModelVariant::Normal,
        "an unmatched recessive WEAK main falls back to NORMAL"
    );
}

#[test]
fn panda_textured_render_matches_vanilla_renderer() {
    // The seven genes × age form a 14-texture matrix; `PandaRenderer.getTextureLocation` keys it off
    // the displayed gene and `isBaby`.
    for (variant, adult, baby) in [
        (
            PandaModelVariant::Normal,
            PANDA_NORMAL_TEXTURE_REF,
            PANDA_NORMAL_BABY_TEXTURE_REF,
        ),
        (
            PandaModelVariant::Lazy,
            PANDA_LAZY_TEXTURE_REF,
            PANDA_LAZY_BABY_TEXTURE_REF,
        ),
        (
            PandaModelVariant::Worried,
            PANDA_WORRIED_TEXTURE_REF,
            PANDA_WORRIED_BABY_TEXTURE_REF,
        ),
        (
            PandaModelVariant::Playful,
            PANDA_PLAYFUL_TEXTURE_REF,
            PANDA_PLAYFUL_BABY_TEXTURE_REF,
        ),
        (
            PandaModelVariant::Brown,
            PANDA_BROWN_TEXTURE_REF,
            PANDA_BROWN_BABY_TEXTURE_REF,
        ),
        (
            PandaModelVariant::Weak,
            PANDA_WEAK_TEXTURE_REF,
            PANDA_WEAK_BABY_TEXTURE_REF,
        ),
        (
            PandaModelVariant::Aggressive,
            PANDA_AGGRESSIVE_TEXTURE_REF,
            PANDA_AGGRESSIVE_BABY_TEXTURE_REF,
        ),
    ] {
        assert_eq!(
            panda_textured_layer_passes(variant, false)[0].texture,
            adult
        );
        assert_eq!(panda_textured_layer_passes(variant, true)[0].texture, baby);
        assert_eq!(
            panda_textured_layer_passes(variant, false)[0].render_type,
            EntityModelLayerRenderType::Cutout
        );
        assert_eq!(
            EntityModelKind::Panda {
                baby: false,
                variant
            }
            .vanilla_texture_ref(),
            Some(adult)
        );
        assert_eq!(
            EntityModelKind::Panda {
                baby: true,
                variant
            }
            .vanilla_texture_ref(),
            Some(baby)
        );
        assert!(entity_model_texture_refs().contains(&adult));
        assert!(entity_model_texture_refs().contains(&baby));
    }
    assert_eq!(panda_entity_texture_refs().len(), 14);

    let images: Vec<EntityModelTextureImage> = panda_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    for baby in [false, true] {
        let mesh = entity_model_textured_mesh(
            &[EntityModelInstance::panda(
                900,
                [0.0, 64.0, 0.0],
                0.0,
                baby,
                PandaModelVariant::Brown,
            )],
            &atlas,
        );
        assert!(
            !mesh.vertices.is_empty(),
            "baby={baby} emits textured geometry"
        );
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    }
}
