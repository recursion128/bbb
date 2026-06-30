use super::*;

use super::super::textured::EntityModelRenderSubmission;
use crate::entity_models::model::ModelCube;

#[test]
fn llama_model_parts_match_vanilla_26_1_body_layers() {
    // The unified cubes carry both render paths' geometry: the colored debug tint and the textured
    // `uv_size`/`texOffs`/`mirror`.
    assert_eq!(
        ADULT_LLAMA_HEAD[0],
        ModelCube::new(
            [-2.0, -14.0, -10.0],
            [4.0, 4.0, 9.0],
            LLAMA_CREAMY,
            [4.0, 4.0, 9.0],
            [0.0, 0.0],
            false,
        )
    );
    assert_eq!(ADULT_LLAMA_HEAD[1].size, [8.0, 18.0, 6.0]);
    assert_eq!(ADULT_LLAMA_BODY[0].size, [12.0, 18.0, 10.0]);
    assert_eq!(ADULT_LLAMA_LEG[0].size, [4.0, 14.0, 4.0]);
    assert_eq!(ADULT_LLAMA_RIGHT_CHEST_POSE.offset, [-8.5, 3.0, 3.0]);
    assert_eq!(
        ADULT_LLAMA_RIGHT_CHEST_POSE.rotation,
        [0.0, std::f32::consts::FRAC_PI_2, 0.0]
    );
    assert_eq!(ADULT_LLAMA_LEFT_CHEST_POSE.offset, [5.5, 3.0, 3.0]);
    assert_eq!(LLAMA_RIGHT_CHEST[0].size, [8.0, 8.0, 3.0]);

    assert_eq!(BABY_LLAMA_HEAD[0].size, [6.0, 11.0, 4.0]);
    assert_eq!(BABY_LLAMA_BODY[0].size, [8.0, 6.0, 13.0]);
    // The baby legs are split right/left by their x-min (right -1.4, left -1.6).
    assert_eq!(BABY_LLAMA_RIGHT_HIND_LEG[0].min[0], -1.4);
    assert_eq!(BABY_LLAMA_LEFT_HIND_LEG[0].min[0], -1.6);
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

    let decor_cases = [
        (EntityDyeColor::White, LLAMA_BODY_WHITE_TEXTURE_REF),
        (EntityDyeColor::Orange, LLAMA_BODY_ORANGE_TEXTURE_REF),
        (EntityDyeColor::Magenta, LLAMA_BODY_MAGENTA_TEXTURE_REF),
        (EntityDyeColor::LightBlue, LLAMA_BODY_LIGHT_BLUE_TEXTURE_REF),
        (EntityDyeColor::Yellow, LLAMA_BODY_YELLOW_TEXTURE_REF),
        (EntityDyeColor::Lime, LLAMA_BODY_LIME_TEXTURE_REF),
        (EntityDyeColor::Pink, LLAMA_BODY_PINK_TEXTURE_REF),
        (EntityDyeColor::Gray, LLAMA_BODY_GRAY_TEXTURE_REF),
        (EntityDyeColor::LightGray, LLAMA_BODY_LIGHT_GRAY_TEXTURE_REF),
        (EntityDyeColor::Cyan, LLAMA_BODY_CYAN_TEXTURE_REF),
        (EntityDyeColor::Purple, LLAMA_BODY_PURPLE_TEXTURE_REF),
        (EntityDyeColor::Blue, LLAMA_BODY_BLUE_TEXTURE_REF),
        (EntityDyeColor::Brown, LLAMA_BODY_BROWN_TEXTURE_REF),
        (EntityDyeColor::Green, LLAMA_BODY_GREEN_TEXTURE_REF),
        (EntityDyeColor::Red, LLAMA_BODY_RED_TEXTURE_REF),
        (EntityDyeColor::Black, LLAMA_BODY_BLACK_TEXTURE_REF),
    ];
    for (color, texture) in decor_cases {
        assert_eq!(llama_body_decor_texture_ref(color), texture);
        assert!(
            entity_model_texture_refs().contains(&texture),
            "{} is included in the global entity atlas",
            texture.path
        );
    }
    assert_eq!(
        LLAMA_BODY_TRADER_TEXTURE_REF,
        EntityModelTextureRef {
            path: "textures/entity/equipment/llama_body/trader_llama.png",
            size: [128, 64],
        }
    );
    assert_eq!(
        LLAMA_BODY_TRADER_BABY_TEXTURE_REF,
        EntityModelTextureRef {
            path: "textures/entity/equipment/llama_body/trader_llama_baby.png",
            size: [64, 64],
        }
    );
    assert!(entity_model_texture_refs().contains(&LLAMA_BODY_TRADER_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&LLAMA_BODY_TRADER_BABY_TEXTURE_REF));
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
    // The trader llama shares the same base mesh/texture; its decor layer is emitted after
    // the base pass, so the base layer remains selected by variant + baby + chest alone.
    let adult = llama_textured_layer_passes(LlamaVariant::Creamy, false, false);
    assert_eq!(adult.len(), 1);
    assert_eq!(adult[0].kind, EntityModelLayerKind::LlamaBase);
    assert_eq!(adult[0].model_layer, MODEL_LAYER_LLAMA);
    assert_eq!(
        adult[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(adult[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(adult[0].texture, LLAMA_CREAMY_TEXTURE_REF);
    assert_eq!(adult[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((adult[0].order, adult[0].submit_sequence), (0, 0));

    let adult_chest = llama_textured_layer_passes(LlamaVariant::White, false, true);
    assert_eq!(adult_chest[0].model_layer, MODEL_LAYER_LLAMA);
    assert_eq!(adult_chest[0].texture, LLAMA_WHITE_TEXTURE_REF);

    let baby = llama_textured_layer_passes(LlamaVariant::Brown, true, false);
    assert_eq!(baby[0].model_layer, MODEL_LAYER_LLAMA_BABY);
    assert_eq!(baby[0].texture, LLAMA_BROWN_BABY_TEXTURE_REF);

    // A baby never shows a chest in vanilla; the chest flag must not change its texture.
    let baby_chest = llama_textured_layer_passes(LlamaVariant::Gray, true, true);
    assert_eq!(baby_chest[0].texture, LLAMA_GRAY_BABY_TEXTURE_REF);
}

#[test]
fn llama_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    // The textured UV sources now live on the unified cubes (`uv_size`/`tex`/`mirror`).
    assert_eq!(MODEL_LAYER_LLAMA, "minecraft:llama#main");
    assert_eq!(MODEL_LAYER_LLAMA_BABY, "minecraft:llama_baby#main");
    assert_eq!(MODEL_LAYER_LLAMA_DECOR, "minecraft:llama#decor");
    assert_eq!(MODEL_LAYER_LLAMA_BABY_DECOR, "minecraft:llama_baby#decor");

    // Adult `LlamaModel.createBodyLayer` (atlas 128×64): head box, neck, the two ears
    // sharing `texOffs(17, 0)` unmirrored, the body, both chests, and the shared leg.
    assert_eq!(ADULT_LLAMA_HEAD[1].uv_size, [8.0, 18.0, 6.0]);
    assert_eq!(ADULT_LLAMA_HEAD[1].tex, [0.0, 14.0]);
    // Both ears share `texOffs(17, 0)`, unmirrored, and the same box size — only their
    // x position differs (right ear at -4, left ear at +1), so they sample the same texels.
    assert_eq!(ADULT_LLAMA_HEAD[2].tex, [17.0, 0.0]);
    assert_eq!(ADULT_LLAMA_HEAD[3].tex, [17.0, 0.0]);
    assert!(!ADULT_LLAMA_HEAD[2].mirror && !ADULT_LLAMA_HEAD[3].mirror);
    assert_eq!(ADULT_LLAMA_HEAD[2].size, ADULT_LLAMA_HEAD[3].size);
    assert_eq!(ADULT_LLAMA_HEAD[2].min[0], -4.0);
    assert_eq!(ADULT_LLAMA_HEAD[3].min[0], 1.0);
    assert_eq!(ADULT_LLAMA_BODY[0].tex, [29.0, 0.0]);
    assert_eq!(LLAMA_RIGHT_CHEST[0].tex, [45.0, 28.0]);
    assert_eq!(LLAMA_LEFT_CHEST[0].tex, [45.0, 41.0]);
    assert_eq!(ADULT_LLAMA_LEG[0].tex, [29.0, 29.0]);

    // Baby `BabyLlamaModel.createBodyLayer` (atlas 64×64): each leg has its own
    // `texOffs` (right/left, hind/front), unlike the adult's single shared leg cube.
    assert_eq!(BABY_LLAMA_HEAD[2].tex, [20.0, 4.0]);
    assert_eq!(BABY_LLAMA_RIGHT_HIND_LEG[0].tex, [0.0, 45.0]);
    assert_eq!(BABY_LLAMA_LEFT_HIND_LEG[0].tex, [12.0, 45.0]);
    assert_eq!(BABY_LLAMA_RIGHT_FRONT_LEG[0].tex, [0.0, 34.0]);
    assert_eq!(BABY_LLAMA_LEFT_FRONT_LEG[0].tex, [12.0, 34.0]);
    assert_eq!(BABY_LLAMA_BODY[0].tex, [0.0, 15.0]);
}

#[test]
fn llama_textured_mesh_renders_vanilla_decor_layer() {
    let images = llama_decor_texture_images();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let adult = EntityModelInstance::llama(
        606,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::Llama,
        LlamaVariant::Creamy,
        false,
        false,
    )
    .with_light_coords((5_u32 << 4) | (11_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true);
    let bare_meshes = entity_model_textured_meshes(&[adult], &atlas);
    assert_eq!(bare_meshes.submissions.len(), 1);
    assert_llama_base_submission_at(&bare_meshes, 0, adult);
    let bare = &bare_meshes.cutout;
    let white_instance = adult.with_llama_body_decor(Some(EntityDyeColor::White));
    let white_meshes = entity_model_textured_meshes(&[white_instance], &atlas);
    let white = &white_meshes.cutout;
    let decor = &white_meshes.armor_cutout;
    assert_eq!(white.vertices.len(), bare.vertices.len());
    assert_eq!(white.indices.len(), bare.indices.len());
    assert_eq!(decor.vertices.len(), bare.vertices.len());
    assert_eq!(decor.indices.len(), bare.indices.len());
    assert_vertex_inside_texture(decor.vertices[0].uv, LLAMA_BODY_WHITE_TEXTURE_REF, &atlas);

    let (bare_min, bare_max) = textured_mesh_extents(&bare);
    let (decor_min, decor_max) = textured_mesh_extents(decor);
    assert!(decor_min[0] < bare_min[0]);
    assert!(decor_max[0] > bare_max[0]);
    assert_eq!(white_meshes.submissions.len(), 2);
    assert_llama_base_submission_at(&white_meshes, 0, white_instance);
    assert_llama_submission(
        white_meshes.submissions[1],
        white_instance,
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        LLAMA_BODY_WHITE_TEXTURE_REF,
        1,
        1,
    );
    assert_ne!(white_instance.render_state.overlay_coords(), [0.0, 10.0]);
    let base_submit = white_meshes.submissions[0];
    let decor_submit = white_meshes.submissions[1];
    assert!(white
        .vertices
        .iter()
        .all(|vertex| vertex.light == base_submit.light && vertex.overlay == base_submit.overlay));
    assert!(
        decor
            .vertices
            .iter()
            .all(|vertex| vertex.light == decor_submit.light
                && vertex.overlay == decor_submit.overlay)
    );

    let adult_trader = EntityModelInstance::llama(
        607,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::TraderLlama,
        LlamaVariant::Creamy,
        false,
        false,
    )
    .with_light_coords((5_u32 << 4) | (11_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true);
    let trader_meshes = entity_model_textured_meshes(&[adult_trader], &atlas);
    let trader = &trader_meshes.cutout;
    let trader_decor = &trader_meshes.armor_cutout;
    assert_eq!(trader.vertices.len(), bare.vertices.len());
    assert_eq!(trader_decor.vertices.len(), bare.vertices.len());
    assert_vertex_inside_texture(
        trader_decor.vertices[0].uv,
        LLAMA_BODY_TRADER_TEXTURE_REF,
        &atlas,
    );
    assert_eq!(trader_meshes.submissions.len(), 2);
    assert_llama_base_submission_at(&trader_meshes, 0, adult_trader);
    assert_llama_submission(
        trader_meshes.submissions[1],
        adult_trader,
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        LLAMA_BODY_TRADER_TEXTURE_REF,
        1,
        1,
    );

    let black_trader_instance = adult_trader.with_llama_body_decor(Some(EntityDyeColor::Black));
    let black_trader_meshes = entity_model_textured_meshes(&[black_trader_instance], &atlas);
    let black_trader = &black_trader_meshes.cutout;
    let black_trader_decor = &black_trader_meshes.armor_cutout;
    assert_eq!(black_trader_meshes.submissions.len(), 2);
    assert_llama_base_submission_at(&black_trader_meshes, 0, black_trader_instance);
    assert_eq!(black_trader.vertices.len(), trader.vertices.len());
    assert_eq!(
        black_trader_decor.vertices.len(),
        trader_decor.vertices.len()
    );
    assert_vertex_inside_texture(
        black_trader_decor.vertices[0].uv,
        LLAMA_BODY_BLACK_TEXTURE_REF,
        &atlas,
    );
    assert_llama_submission(
        black_trader_meshes.submissions[1],
        black_trader_instance,
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        LLAMA_BODY_BLACK_TEXTURE_REF,
        1,
        1,
    );

    let baby = EntityModelInstance::llama(
        608,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::Llama,
        LlamaVariant::Creamy,
        true,
        false,
    )
    .with_light_coords((5_u32 << 4) | (11_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true);
    let baby_bare_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_eq!(baby_bare_meshes.submissions.len(), 1);
    assert_llama_base_submission_at(&baby_bare_meshes, 0, baby);
    let baby_bare = &baby_bare_meshes.cutout;
    let baby_with_decor_instance = baby.with_llama_body_decor(Some(EntityDyeColor::White));
    let baby_with_decor_meshes = entity_model_textured_meshes(&[baby_with_decor_instance], &atlas);
    let baby_with_decor = &baby_with_decor_meshes.cutout;
    assert_eq!(
        baby_with_decor.vertices, baby_bare.vertices,
        "baby llamas ignore carpet body equipment"
    );
    assert_eq!(baby_with_decor_meshes.submissions.len(), 1);
    assert_llama_base_submission_at(&baby_with_decor_meshes, 0, baby_with_decor_instance);

    let baby_trader = EntityModelInstance::llama(
        609,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::TraderLlama,
        LlamaVariant::Creamy,
        true,
        false,
    )
    .with_light_coords((5_u32 << 4) | (11_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true)
    .with_llama_body_decor(Some(EntityDyeColor::Black));
    let baby_trader_meshes = entity_model_textured_meshes(&[baby_trader], &atlas);
    assert_eq!(baby_trader_meshes.submissions.len(), 2);
    assert_llama_base_submission_at(&baby_trader_meshes, 0, baby_trader);
    let baby_trader_mesh = &baby_trader_meshes.cutout;
    let baby_trader_decor = &baby_trader_meshes.armor_cutout;
    assert_eq!(baby_trader_mesh.vertices.len(), baby_bare.vertices.len());
    assert_eq!(baby_trader_decor.vertices.len(), baby_bare.vertices.len());
    assert_vertex_inside_texture(
        baby_trader_decor.vertices[0].uv,
        LLAMA_BODY_TRADER_BABY_TEXTURE_REF,
        &atlas,
    );
    assert_llama_submission(
        baby_trader_meshes.submissions[1],
        baby_trader,
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        LLAMA_BODY_TRADER_BABY_TEXTURE_REF,
        1,
        1,
    );
}

#[test]
fn llama_decor_submission_survives_missing_texture_atlas_entry() {
    // Vanilla `LlamaDecorLayer` delegates to `EquipmentLayerRenderer.renderLayers(LLAMA_BODY)`,
    // whose public overload starts at `SubmitNodeCollector.order(1)` and uses no overlay.
    let images = texture_images(&[LLAMA_CREAMY_TEXTURE_REF, LLAMA_CREAMY_BABY_TEXTURE_REF]);
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let adult = EntityModelInstance::llama(
        610,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::Llama,
        LlamaVariant::Creamy,
        false,
        false,
    )
    .with_light_coords((5_u32 << 4) | (11_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true);
    let bare_meshes = entity_model_textured_meshes(&[adult], &atlas);
    assert_llama_base_submission_at(&bare_meshes, 0, adult);

    let white_instance = adult.with_llama_body_decor(Some(EntityDyeColor::White));
    let white_meshes = entity_model_textured_meshes(&[white_instance], &atlas);
    assert_eq!(white_meshes.submissions.len(), 2);
    assert_llama_base_submission_at(&white_meshes, 0, white_instance);
    assert_llama_submission(
        white_meshes.submissions[1],
        white_instance,
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        LLAMA_BODY_WHITE_TEXTURE_REF,
        1,
        1,
    );
    assert_eq!(
        white_meshes.cutout.vertices, bare_meshes.cutout.vertices,
        "missing llama_body/white.png suppresses only folded decor geometry"
    );
    assert_eq!(white_meshes.cutout.indices, bare_meshes.cutout.indices);
    let base_submit = white_meshes.submissions[0];
    assert!(white_meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == base_submit.light && vertex.overlay == base_submit.overlay));

    let baby_bare = EntityModelInstance::llama(
        611,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::Llama,
        LlamaVariant::Creamy,
        true,
        false,
    )
    .with_light_coords((5_u32 << 4) | (11_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true);
    let baby_bare_meshes = entity_model_textured_meshes(&[baby_bare], &atlas);
    assert_llama_base_submission_at(&baby_bare_meshes, 0, baby_bare);
    let baby_trader = EntityModelInstance::llama(
        612,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::TraderLlama,
        LlamaVariant::Creamy,
        true,
        false,
    )
    .with_light_coords((5_u32 << 4) | (11_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true)
    .with_llama_body_decor(Some(EntityDyeColor::Black));
    let baby_trader_meshes = entity_model_textured_meshes(&[baby_trader], &atlas);
    assert_eq!(baby_trader_meshes.submissions.len(), 2);
    assert_llama_base_submission_at(&baby_trader_meshes, 0, baby_trader);
    assert_llama_submission(
        baby_trader_meshes.submissions[1],
        baby_trader,
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        LLAMA_BODY_TRADER_BABY_TEXTURE_REF,
        1,
        1,
    );
    assert_eq!(
        baby_trader_meshes.cutout.vertices, baby_bare_meshes.cutout.vertices,
        "missing llama_body/trader_llama_baby.png suppresses only folded trader decor geometry"
    );
    assert_eq!(
        baby_trader_meshes.cutout.indices,
        baby_bare_meshes.cutout.indices
    );
    assert!(baby_trader_meshes
        .cutout
        .vertices
        .iter()
        .all(
            |vertex| vertex.light == baby_trader_meshes.submissions[0].light
                && vertex.overlay == baby_trader_meshes.submissions[0].overlay
        ));
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
    let yawed_instance = base.with_head_look(45.0, 0.0);
    let pitched_instance = base.with_head_look(0.0, -25.0);
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let yawed = entity_model_textured_meshes(&[yawed_instance], &atlas);
    let pitched = entity_model_textured_meshes(&[pitched_instance], &atlas);
    assert_eq!(resting.submissions.len(), 1);
    assert_eq!(yawed.submissions.len(), 1);
    assert_eq!(pitched.submissions.len(), 1);
    assert_llama_base_submission_at(&resting, 0, base);
    assert_llama_base_submission_at(&yawed, 0, yawed_instance);
    assert_llama_base_submission_at(&pitched, 0, pitched_instance);

    // Head look turns the textured head part without adding or dropping vertices.
    assert_eq!(resting.cutout.vertices.len(), yawed.cutout.vertices.len());
    assert_ne!(resting.cutout.vertices, yawed.cutout.vertices);
    assert_ne!(resting.cutout.vertices, pitched.cutout.vertices);
    assert_ne!(yawed.cutout.vertices, pitched.cutout.vertices);
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
        let still_instance = base.with_walk_animation(2.5, 0.0);
        let walking_instance = base.with_walk_animation(0.0, 1.0);
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let still = entity_model_textured_meshes(&[still_instance], &atlas);
        let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
        assert_eq!(resting.submissions.len(), 1);
        assert_eq!(still.submissions.len(), 1);
        assert_eq!(walking.submissions.len(), 1);
        assert_llama_base_submission_at(&resting, 0, base);
        assert_llama_base_submission_at(&still, 0, still_instance);
        assert_llama_base_submission_at(&walking, 0, walking_instance);

        assert_eq!(
            resting.cutout.vertices, still.cutout.vertices,
            "{variant:?} baby={baby} chest={has_chest}: a standing llama is inert"
        );
        assert_eq!(
            resting.cutout.vertices.len(),
            walking.cutout.vertices.len(),
            "{variant:?} baby={baby} chest={has_chest}: leg swing keeps the vertex count"
        );
        assert_ne!(
            resting.cutout.vertices, walking.cutout.vertices,
            "{variant:?} baby={baby} chest={has_chest}: a walking llama differs"
        );

        let (rest_min, rest_max) = textured_mesh_extents(&resting.cutout);
        let (walk_min, walk_max) = textured_mesh_extents(&walking.cutout);
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
            "{variant:?} baby={baby} chest={has_chest}: a walking llama's feet lift off"
        );
    }
}

fn assert_llama_base_submission_at(
    meshes: &EntityModelTexturedMeshes,
    index: usize,
    instance: EntityModelInstance,
) {
    assert_llama_folded_meshes_are_cutout_only(meshes);
    assert_llama_submission(
        meshes.submissions[index],
        instance,
        EntityModelLayerRenderType::EntityCutout,
        instance
            .kind
            .vanilla_texture_ref()
            .expect("llama base texture ref"),
        0,
        u32::try_from(index).expect("submission index fits in u32"),
    );
}

fn assert_llama_submission(
    submit: EntityModelRenderSubmission,
    instance: EntityModelInstance,
    render_type: EntityModelLayerRenderType,
    texture: EntityModelTextureRef,
    order: i32,
    submit_sequence: u32,
) {
    let expected_render_type_name = match render_type {
        EntityModelLayerRenderType::EntityCutout => "entityCutout",
        EntityModelLayerRenderType::ArmorCutoutNoCull => "armorCutoutNoCull",
        other => panic!("unexpected llama render type {other:?}"),
    };
    assert_eq!(submit.render_type, render_type);
    assert_eq!(submit.render_type.vanilla_name(), expected_render_type_name);
    assert_eq!(submit.texture, texture);
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(submit.transform, entity_model_root_transform(instance));
    assert_eq!(
        (submit.order, submit.submit_sequence),
        (order, submit_sequence)
    );
    assert_eq!(submit.light, instance.render_state.shader_light());
    assert_eq!(
        submit.overlay,
        match render_type {
            EntityModelLayerRenderType::EntityCutout => instance.render_state.overlay_coords(),
            EntityModelLayerRenderType::ArmorCutoutNoCull => [0.0, 10.0],
            other => panic!("unexpected llama render type {other:?}"),
        }
    );
}

fn assert_llama_folded_meshes_are_cutout_only(meshes: &EntityModelTexturedMeshes) {
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
    assert!(meshes.scroll_additive.vertices.is_empty());
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

fn llama_decor_texture_images() -> Vec<EntityModelTextureImage> {
    texture_images(&[
        LLAMA_CREAMY_TEXTURE_REF,
        LLAMA_CREAMY_BABY_TEXTURE_REF,
        LLAMA_BODY_WHITE_TEXTURE_REF,
        LLAMA_BODY_BLACK_TEXTURE_REF,
        LLAMA_BODY_TRADER_TEXTURE_REF,
        LLAMA_BODY_TRADER_BABY_TEXTURE_REF,
    ])
}

fn texture_images(textures: &[EntityModelTextureRef]) -> Vec<EntityModelTextureImage> {
    textures
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![(index * 40) as u8; len])
        })
        .collect()
}

fn assert_vertex_inside_texture(
    uv: [f32; 2],
    texture: EntityModelTextureRef,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let entry = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == texture)
        .unwrap();
    assert!(uv[0] >= entry.uv.min[0]);
    assert!(uv[0] <= entry.uv.max[0]);
    assert!(uv[1] >= entry.uv.min[1]);
    assert!(uv[1] <= entry.uv.max[1]);
}
