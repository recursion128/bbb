use super::*;

#[test]
fn llama_model_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(
        ADULT_LLAMA_HEAD[0],
        ModelCubeDesc {
            min: [-2.0, -14.0, -10.0],
            size: [4.0, 4.0, 9.0],
            color: LLAMA_CREAMY,
        }
    );
    assert_eq!(
        ADULT_LLAMA_HEAD[1],
        ModelCubeDesc {
            min: [-4.0, -16.0, -6.0],
            size: [8.0, 18.0, 6.0],
            color: LLAMA_CREAMY,
        }
    );
    assert_eq!(ADULT_LLAMA_PARTS.len(), 6);
    assert_part(
        &ADULT_LLAMA_PARTS[0],
        [0.0, 7.0, -6.0],
        [0.0, 0.0, 0.0],
        ADULT_LLAMA_HEAD.as_slice(),
    );
    assert_part(
        &ADULT_LLAMA_PARTS[1],
        [0.0, 5.0, 2.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_LLAMA_BODY.as_slice(),
    );
    assert_part(
        &ADULT_LLAMA_RIGHT_CHEST_PART,
        [-8.5, 3.0, 3.0],
        [0.0, std::f32::consts::FRAC_PI_2, 0.0],
        LLAMA_CHEST.as_slice(),
    );
    assert_part(
        &ADULT_LLAMA_LEFT_CHEST_PART,
        [5.5, 3.0, 3.0],
        [0.0, std::f32::consts::FRAC_PI_2, 0.0],
        LLAMA_CHEST.as_slice(),
    );
    assert_eq!(ADULT_LLAMA_PARTS_WITH_CHEST.len(), 8);
    for (part, expected_offset) in [
        (&ADULT_LLAMA_PARTS[2], [-3.5, 10.0, 6.0]),
        (&ADULT_LLAMA_PARTS[3], [3.5, 10.0, 6.0]),
        (&ADULT_LLAMA_PARTS[4], [-3.5, 10.0, -5.0]),
        (&ADULT_LLAMA_PARTS[5], [3.5, 10.0, -5.0]),
    ] {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            ADULT_LLAMA_LEG.as_slice(),
        );
    }

    assert_eq!(
        BABY_LLAMA_HEAD[0],
        ModelCubeDesc {
            min: [-3.0, -9.0, -4.0],
            size: [6.0, 11.0, 4.0],
            color: LLAMA_CREAMY,
        }
    );
    assert_eq!(BABY_LLAMA_PARTS.len(), 6);
    assert_part(
        &BABY_LLAMA_PARTS[0],
        [0.0, 12.0, -4.0],
        [0.0, 0.0, 0.0],
        BABY_LLAMA_HEAD.as_slice(),
    );
    assert_part(
        &BABY_LLAMA_PARTS[5],
        [0.0, 14.0, 2.5],
        [0.0, 0.0, 0.0],
        BABY_LLAMA_BODY.as_slice(),
    );
    for (part, expected_offset, expected_cubes) in [
        (
            &BABY_LLAMA_PARTS[1],
            [-2.5, 16.5, 4.5],
            BABY_LLAMA_RIGHT_LEG.as_slice(),
        ),
        (
            &BABY_LLAMA_PARTS[2],
            [2.5, 16.5, 4.5],
            BABY_LLAMA_LEFT_LEG.as_slice(),
        ),
        (
            &BABY_LLAMA_PARTS[3],
            [-2.5, 16.5, -3.5],
            BABY_LLAMA_RIGHT_LEG.as_slice(),
        ),
        (
            &BABY_LLAMA_PARTS[4],
            [2.5, 16.5, -3.5],
            BABY_LLAMA_LEFT_LEG.as_slice(),
        ),
    ] {
        assert_part(part, expected_offset, [0.0, 0.0, 0.0], expected_cubes);
    }
}

#[test]
fn llama_meshes_use_vanilla_body_layer_geometry_and_chest_visibility() {
    let adult = entity_model_mesh(&[EntityModelInstance::llama(
        190,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::Llama,
        LlamaVariant::Creamy,
        false,
        false,
    )]);
    assert_eq!(adult.opaque_faces, 54);
    assert_eq!(adult.vertices.len(), 216);
    assert_eq!(adult.indices.len(), 324);

    let adult_with_chest = entity_model_mesh(&[EntityModelInstance::llama(
        191,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::Llama,
        LlamaVariant::Brown,
        false,
        true,
    )]);
    assert_eq!(adult_with_chest.opaque_faces, 66);
    assert_eq!(adult_with_chest.vertices.len(), 264);
    assert_eq!(adult_with_chest.indices.len(), 396);
    assert!(adult_with_chest
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(LLAMA_BROWN, 0.78)));

    let baby = entity_model_mesh(&[EntityModelInstance::llama(
        192,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::Llama,
        LlamaVariant::Gray,
        true,
        false,
    )]);
    assert_eq!(baby.opaque_faces, 54);
    assert_eq!(baby.vertices.len(), 216);
    assert_eq!(baby.indices.len(), 324);
    assert!(baby
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(LLAMA_GRAY, 0.78)));

    let baby_with_chest_metadata = entity_model_mesh(&[EntityModelInstance::llama(
        193,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::TraderLlama,
        LlamaVariant::Gray,
        true,
        true,
    )]);
    assert_same_geometry(&baby_with_chest_metadata, &baby);

    let trader = entity_model_mesh(&[EntityModelInstance::llama(
        194,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::TraderLlama,
        LlamaVariant::Creamy,
        false,
        false,
    )]);
    assert_same_geometry(&trader, &adult);

    let (adult_min, adult_max) = mesh_extents(&adult);
    let (baby_min, baby_max) = mesh_extents(&baby);
    assert!(adult_max[1] > baby_max[1]);
    assert!(adult_min[2] < baby_min[2]);
}

#[test]
fn llama_texture_refs_match_vanilla_renderer() {
    let cases = [
        (
            LlamaVariant::Creamy,
            false,
            EntityModelTextureRef {
                path: "textures/entity/llama/llama_creamy.png",
                size: [128, 64],
            },
        ),
        (
            LlamaVariant::Creamy,
            true,
            EntityModelTextureRef {
                path: "textures/entity/llama/llama_creamy_baby.png",
                size: [64, 64],
            },
        ),
        (
            LlamaVariant::White,
            false,
            EntityModelTextureRef {
                path: "textures/entity/llama/llama_white.png",
                size: [128, 64],
            },
        ),
        (
            LlamaVariant::White,
            true,
            EntityModelTextureRef {
                path: "textures/entity/llama/llama_white_baby.png",
                size: [64, 64],
            },
        ),
        (
            LlamaVariant::Brown,
            false,
            EntityModelTextureRef {
                path: "textures/entity/llama/llama_brown.png",
                size: [128, 64],
            },
        ),
        (
            LlamaVariant::Brown,
            true,
            EntityModelTextureRef {
                path: "textures/entity/llama/llama_brown_baby.png",
                size: [64, 64],
            },
        ),
        (
            LlamaVariant::Gray,
            false,
            EntityModelTextureRef {
                path: "textures/entity/llama/llama_gray.png",
                size: [128, 64],
            },
        ),
        (
            LlamaVariant::Gray,
            true,
            EntityModelTextureRef {
                path: "textures/entity/llama/llama_gray_baby.png",
                size: [64, 64],
            },
        ),
    ];

    for (variant, baby, texture) in cases {
        let llama = EntityModelKind::Llama {
            family: LlamaModelFamily::Llama,
            variant,
            baby,
            has_chest: false,
        };
        let trader = EntityModelKind::Llama {
            family: LlamaModelFamily::TraderLlama,
            variant,
            baby,
            has_chest: false,
        };
        assert_eq!(llama.vanilla_texture_ref(), Some(texture));
        assert_eq!(trader.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn llama_swings_its_legs_when_walking() {
    // Vanilla `LlamaModel.setupAnim` swings the four legs with the standard
    // `QuadrupedModel` diagonal phase `cos(pos * 0.6662 [+ π]) * 1.4 * speed` (right-hind/
    // left-front in phase). A standing llama is inert; a walking one lifts its feet and
    // splays its legs along Z. The chest layout (legs pushed to [4, 5, 6, 7]) and the
    // baby layout (legs at [1, 2, 3, 4]) both swing only their legs, not the chests/body.
    for base in [
        EntityModelInstance::llama(
            190,
            [0.0, 64.0, 0.0],
            0.0,
            LlamaModelFamily::Llama,
            LlamaVariant::Creamy,
            false,
            false,
        ),
        EntityModelInstance::llama(
            191,
            [0.0, 64.0, 0.0],
            0.0,
            LlamaModelFamily::Llama,
            LlamaVariant::Brown,
            false,
            true,
        ),
        EntityModelInstance::llama(
            192,
            [0.0, 64.0, 0.0],
            0.0,
            LlamaModelFamily::Llama,
            LlamaVariant::Gray,
            true,
            false,
        ),
    ] {
        let rest = entity_model_mesh(&[base]);
        let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
        assert_eq!(
            rest.vertices, still.vertices,
            "{:?} rest is inert",
            base.kind
        );

        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_ne!(
            rest.vertices, walking.vertices,
            "{:?} walking differs",
            base.kind
        );

        let (rest_min, rest_max) = mesh_extents(&rest);
        let (walk_min, walk_max) = mesh_extents(&walking);
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
            "{:?} feet should lift off the ground",
            base.kind
        );
        assert!(
            (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
            "{:?} legs should splay along Z",
            base.kind
        );
    }
}

#[test]
fn llama_applies_head_look() {
    // Vanilla `LlamaModel.setupAnim` sets `head.xRot = pitch`, `head.yRot = yaw` on the
    // head part (index 0 in every layout). Turning or pitching the head changes the mesh.
    for base in [
        EntityModelInstance::llama(
            195,
            [0.0, 64.0, 0.0],
            0.0,
            LlamaModelFamily::Llama,
            LlamaVariant::Creamy,
            false,
            false,
        ),
        EntityModelInstance::llama(
            196,
            [0.0, 64.0, 0.0],
            0.0,
            LlamaModelFamily::TraderLlama,
            LlamaVariant::Gray,
            true,
            false,
        ),
    ] {
        let resting = entity_model_mesh(&[base]);
        let yawed = entity_model_mesh(&[base.with_head_look(40.0, 0.0)]);
        let pitched = entity_model_mesh(&[base.with_head_look(0.0, -25.0)]);
        assert_eq!(resting.vertices.len(), yawed.vertices.len());
        assert_ne!(resting.vertices, yawed.vertices, "{:?} head yaw", base.kind);
        assert_ne!(
            yawed.vertices, pitched.vertices,
            "{:?} head pitch",
            base.kind
        );
    }
}

#[test]
fn llama_textured_layer_passes_match_vanilla_renderer_model_choice() {
    // The trader llama shares the same base mesh/texture; only its deferred decor layer
    // differs, so the textured base layer is selected by variant + baby + chest alone.
    let adult = llama_textured_layer_passes(LlamaVariant::Creamy, false, false);
    assert_eq!(adult.len(), 1);
    assert_eq!(adult[0].kind, EntityModelLayerKind::LlamaBase);
    assert_eq!(adult[0].model_layer, MODEL_LAYER_LLAMA);
    assert_eq!(adult[0].texture, LLAMA_CREAMY_TEXTURE_REF);
    assert_eq!(adult[0].parts, ADULT_LLAMA_TEXTURED_PARTS.as_slice());
    assert_eq!(adult[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((adult[0].collector_order, adult[0].submit_sequence), (0, 0));

    let adult_chest = llama_textured_layer_passes(LlamaVariant::White, false, true);
    assert_eq!(adult_chest[0].model_layer, MODEL_LAYER_LLAMA);
    assert_eq!(adult_chest[0].texture, LLAMA_WHITE_TEXTURE_REF);
    assert_eq!(
        adult_chest[0].parts,
        ADULT_LLAMA_TEXTURED_PARTS_WITH_CHEST.as_slice()
    );

    let baby = llama_textured_layer_passes(LlamaVariant::Brown, true, false);
    assert_eq!(baby[0].model_layer, MODEL_LAYER_LLAMA_BABY);
    assert_eq!(baby[0].texture, LLAMA_BROWN_BABY_TEXTURE_REF);
    assert_eq!(baby[0].parts, BABY_LLAMA_TEXTURED_PARTS.as_slice());

    // A baby never shows a chest in vanilla; the chest flag must not change its parts.
    let baby_chest = llama_textured_layer_passes(LlamaVariant::Gray, true, true);
    assert_eq!(baby_chest[0].texture, LLAMA_GRAY_BABY_TEXTURE_REF);
    assert_eq!(baby_chest[0].parts, BABY_LLAMA_TEXTURED_PARTS.as_slice());
}

#[test]
fn llama_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_LLAMA, "minecraft:llama#main");
    assert_eq!(MODEL_LAYER_LLAMA_BABY, "minecraft:llama_baby#main");

    // Adult `LlamaModel.createBodyLayer` (atlas 128×64): head box, neck, the two ears
    // sharing `texOffs(17, 0)` unmirrored, the body, both chests, and the shared leg.
    assert_eq!(
        ADULT_LLAMA_TEXTURED_HEAD[1],
        TexturedModelCubeDesc {
            min: [-4.0, -16.0, -6.0],
            size: [8.0, 18.0, 6.0],
            uv_size: [8.0, 18.0, 6.0],
            tex: [0.0, 14.0],
            mirror: false,
        }
    );
    // Both ears share `texOffs(17, 0)`, unmirrored, and the same box size — only their
    // x position differs (right ear at -4, left ear at +1), so they sample the same texels.
    assert_eq!(ADULT_LLAMA_TEXTURED_HEAD[2].tex, [17.0, 0.0]);
    assert_eq!(ADULT_LLAMA_TEXTURED_HEAD[3].tex, [17.0, 0.0]);
    assert!(!ADULT_LLAMA_TEXTURED_HEAD[2].mirror && !ADULT_LLAMA_TEXTURED_HEAD[3].mirror);
    assert_eq!(
        ADULT_LLAMA_TEXTURED_HEAD[2].size,
        ADULT_LLAMA_TEXTURED_HEAD[3].size
    );
    assert_eq!(ADULT_LLAMA_TEXTURED_HEAD[2].min[0], -4.0);
    assert_eq!(ADULT_LLAMA_TEXTURED_HEAD[3].min[0], 1.0);
    assert_eq!(
        ADULT_LLAMA_TEXTURED_BODY[0],
        TexturedModelCubeDesc {
            min: [-6.0, -10.0, -7.0],
            size: [12.0, 18.0, 10.0],
            uv_size: [12.0, 18.0, 10.0],
            tex: [29.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(ADULT_LLAMA_TEXTURED_RIGHT_CHEST[0].tex, [45.0, 28.0]);
    assert_eq!(ADULT_LLAMA_TEXTURED_LEFT_CHEST[0].tex, [45.0, 41.0]);
    assert_eq!(
        ADULT_LLAMA_TEXTURED_LEG[0],
        TexturedModelCubeDesc {
            min: [-2.0, 0.0, -2.0],
            size: [4.0, 14.0, 4.0],
            uv_size: [4.0, 14.0, 4.0],
            tex: [29.0, 29.0],
            mirror: false,
        }
    );

    // Baby `BabyLlamaModel.createBodyLayer` (atlas 64×64): each leg has its own
    // `texOffs` (right/left, hind/front), unlike the adult's single shared leg cube.
    assert_eq!(
        BABY_LLAMA_TEXTURED_HEAD[2],
        TexturedModelCubeDesc {
            min: [0.5, -11.0, -3.0],
            size: [2.0, 2.0, 2.0],
            uv_size: [2.0, 2.0, 2.0],
            tex: [20.0, 4.0],
            mirror: false,
        }
    );
    assert_eq!(BABY_LLAMA_TEXTURED_RIGHT_HIND_LEG[0].tex, [0.0, 45.0]);
    assert_eq!(BABY_LLAMA_TEXTURED_LEFT_HIND_LEG[0].tex, [12.0, 45.0]);
    assert_eq!(BABY_LLAMA_TEXTURED_RIGHT_FRONT_LEG[0].tex, [0.0, 34.0]);
    assert_eq!(BABY_LLAMA_TEXTURED_LEFT_FRONT_LEG[0].tex, [12.0, 34.0]);
    assert_eq!(BABY_LLAMA_TEXTURED_RIGHT_HIND_LEG[0].min[0], -1.4);
    assert_eq!(BABY_LLAMA_TEXTURED_LEFT_HIND_LEG[0].min[0], -1.6);
    assert_eq!(
        BABY_LLAMA_TEXTURED_BODY[0],
        TexturedModelCubeDesc {
            min: [-4.0, -3.0, -8.5],
            size: [8.0, 6.0, 13.0],
            uv_size: [8.0, 6.0, 13.0],
            tex: [0.0, 15.0],
            mirror: false,
        }
    );

    // The textured part trees reuse the colored part poses, in the colored layouts.
    assert_eq!(ADULT_LLAMA_TEXTURED_PARTS.len(), 6);
    assert_eq!(ADULT_LLAMA_TEXTURED_PARTS_WITH_CHEST.len(), 8);
    assert_eq!(
        ADULT_LLAMA_TEXTURED_PARTS[0].pose,
        ADULT_LLAMA_PARTS[0].pose
    );
    assert_eq!(
        ADULT_LLAMA_TEXTURED_PARTS_WITH_CHEST[2].pose,
        ADULT_LLAMA_RIGHT_CHEST_PART.pose
    );
    assert_eq!(BABY_LLAMA_TEXTURED_PARTS.len(), 6);
    assert_eq!(BABY_LLAMA_TEXTURED_PARTS[5].pose, BABY_LLAMA_PARTS[5].pose);
}

#[test]
fn llama_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&llama_texture_images()).unwrap();
    let base = EntityModelInstance::llama(
        604,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::Llama,
        LlamaVariant::Creamy,
        false,
        false,
    );
    let resting = entity_model_textured_mesh(&[base], &atlas);
    let yawed = entity_model_textured_mesh(&[base.with_head_look(45.0, 0.0)], &atlas);
    let pitched = entity_model_textured_mesh(&[base.with_head_look(0.0, -25.0)], &atlas);

    // Head look turns the textured head part without adding or dropping vertices.
    assert_eq!(resting.vertices.len(), yawed.vertices.len());
    assert_ne!(resting.vertices, yawed.vertices);
    assert_ne!(resting.vertices, pitched.vertices);
    assert_ne!(yawed.vertices, pitched.vertices);
}

#[test]
fn llama_textured_mesh_swings_legs_when_walking() {
    // The textured llama render path consumes the projected limb swing via the vanilla
    // QuadrupedModel diagonal leg rotation. A standing llama is byte-identical however
    // far the swing position has advanced; a walking one lifts its feet off the ground,
    // for the adult, the with-chest, and the baby layouts (legs [2..5]/[4..7]/[1..4]).
    let (atlas, _) = build_entity_model_texture_atlas(&llama_texture_images()).unwrap();
    for (variant, baby, has_chest) in [
        (LlamaVariant::Creamy, false, false),
        (LlamaVariant::Brown, false, true),
        (LlamaVariant::Gray, true, false),
    ] {
        let base = EntityModelInstance::llama(
            605,
            [0.0, 64.0, 0.0],
            0.0,
            LlamaModelFamily::Llama,
            variant,
            baby,
            has_chest,
        );
        let resting = entity_model_textured_mesh(&[base], &atlas);
        let still = entity_model_textured_mesh(&[base.with_walk_animation(2.5, 0.0)], &atlas);
        let walking = entity_model_textured_mesh(&[base.with_walk_animation(0.0, 1.0)], &atlas);

        assert_eq!(
            resting.vertices, still.vertices,
            "{variant:?} baby={baby} chest={has_chest}: a standing llama is inert"
        );
        assert_eq!(
            resting.vertices.len(),
            walking.vertices.len(),
            "{variant:?} baby={baby} chest={has_chest}: leg swing keeps the vertex count"
        );
        assert_ne!(
            resting.vertices, walking.vertices,
            "{variant:?} baby={baby} chest={has_chest}: a walking llama differs"
        );

        let (rest_min, rest_max) = textured_mesh_extents(&resting);
        let (walk_min, walk_max) = textured_mesh_extents(&walking);
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
            "{variant:?} baby={baby} chest={has_chest}: a walking llama's feet lift off"
        );
    }
}

fn llama_texture_images() -> Vec<EntityModelTextureImage> {
    llama_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
