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
