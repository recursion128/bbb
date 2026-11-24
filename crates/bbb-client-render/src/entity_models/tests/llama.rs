use super::*;

fn entity_model_textured_meshes(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> EntityModelTexturedMeshes {
    super::entity_model_textured_meshes_with_static_equipment_fixture(instances, atlas)
}

fn entity_model_textured_meshes_with_dynamic_textures(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
    skins: Option<&EntityDynamicPlayerSkinAtlasLayout>,
    textures: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) -> EntityModelTexturedMeshes {
    super::entity_model_textured_meshes_with_static_equipment_and_dynamic_textures_fixture(
        instances, atlas, skins, textures,
    )
}

use crate::DynamicEntityTextureImage;

#[test]
fn llama_texture_refs_match_vanilla_renderer() {
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
    for (_color, texture) in decor_cases {
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
fn llama_textured_layer_passes_match_vanilla_renderer_model_choice() {
    // The trader llama shares the same base mesh/texture; its decor layer is emitted after
    // the base pass, so the base layer remains selected by variant + baby + chest alone.
    let adult = llama_textured_layer_passes(LlamaVariant::Creamy, false, false);
    assert_eq!(adult.len(), 1);
    assert_eq!(
        adult[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(adult[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(adult[0].texture, LLAMA_CREAMY_TEXTURE_REF);
    assert_eq!(adult[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(adult[0].order, 0);

    let adult_chest = llama_textured_layer_passes(LlamaVariant::White, false, true);
    assert_eq!(adult_chest[0].texture, LLAMA_WHITE_TEXTURE_REF);

    let baby = llama_textured_layer_passes(LlamaVariant::Brown, true, false);
    assert_eq!(baby[0].texture, LLAMA_BROWN_BABY_TEXTURE_REF);

    // A baby never shows a chest in vanilla; the chest flag must not change its texture.
    let baby_chest = llama_textured_layer_passes(LlamaVariant::Gray, true, true);
    assert_eq!(baby_chest[0].texture, LLAMA_GRAY_BABY_TEXTURE_REF);
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
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let bare_meshes = entity_model_textured_meshes(&[adult], &atlas);

    assert_llama_folded_meshes_are_cutout_only(&bare_meshes);
    let bare = &bare_meshes.cutout;
    let white_instance = adult.with(|s| {
        s.kind.llama_mut().body_equipment_asset = Some(test_llama_white_equipment_asset_id())
    });
    let white_meshes = entity_model_textured_meshes(&[white_instance], &atlas);
    let white = &white_meshes.cutout;
    let decor = &white_meshes.armor_cutout;
    assert_eq!(white.vertices.len(), bare.vertices.len());
    assert_eq!(white.indices.len(), bare.indices.len());
    assert_eq!(decor.vertices.len(), bare.vertices.len());
    assert_eq!(decor.indices.len(), bare.indices.len());
    assert_vertex_inside_texture(decor.vertices[0].uv, LLAMA_BODY_WHITE_TEXTURE_REF, &atlas);

    let (bare_min, bare_max) = textured_mesh_extents(bare);
    let (decor_min, decor_max) = textured_mesh_extents(decor);
    assert!(decor_min[0] < bare_min[0]);
    assert!(decor_max[0] > bare_max[0]);

    assert_llama_folded_meshes_are_cutout_only(&white_meshes);

    assert_ne!(white_instance.render_state.overlay_coords(), [0.0, 10.0]);

    assert!(white.vertices.iter().all(|vertex| vertex.light
        == white_instance.render_state.shader_light()
        && vertex.overlay == white_instance.render_state.overlay_coords()));
    assert!(decor.vertices.iter().all(|vertex| vertex.light
        == white_instance.render_state.shader_light()
        && vertex.overlay == [0.0, 10.0]));

    let adult_trader = EntityModelInstance::llama(
        607,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::TraderLlama,
        LlamaVariant::Creamy,
        false,
        false,
    )
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.kind.llama_mut().body_equipment_asset = Some(test_llama_trader_equipment_asset_id());
    });
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

    assert_llama_folded_meshes_are_cutout_only(&trader_meshes);

    let black_trader_instance = adult_trader.with(|s| {
        s.kind.llama_mut().body_equipment_asset = Some(test_llama_black_equipment_asset_id())
    });
    let black_trader_meshes = entity_model_textured_meshes(&[black_trader_instance], &atlas);
    let black_trader = &black_trader_meshes.cutout;
    let black_trader_decor = &black_trader_meshes.armor_cutout;

    assert_llama_folded_meshes_are_cutout_only(&black_trader_meshes);
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

    let baby = EntityModelInstance::llama(
        608,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::Llama,
        LlamaVariant::Creamy,
        true,
        false,
    )
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let baby_bare_meshes = entity_model_textured_meshes(&[baby], &atlas);

    assert_llama_folded_meshes_are_cutout_only(&baby_bare_meshes);
    let baby_bare = &baby_bare_meshes.cutout;
    let baby_with_decor_instance = baby.with(|s| {
        s.kind.llama_mut().body_equipment_asset = Some(test_llama_white_equipment_asset_id())
    });
    let baby_with_decor_meshes = entity_model_textured_meshes(&[baby_with_decor_instance], &atlas);
    let baby_with_decor = &baby_with_decor_meshes.cutout;
    assert_eq!(
        baby_with_decor.vertices, baby_bare.vertices,
        "equipment projection must not alter the base baby llama mesh"
    );

    assert_llama_folded_meshes_are_cutout_only(&baby_with_decor_meshes);

    let baby_trader = EntityModelInstance::llama(
        609,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::TraderLlama,
        LlamaVariant::Creamy,
        true,
        false,
    )
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.kind.llama_mut().body_equipment_asset = Some(test_llama_trader_baby_equipment_asset_id());
    });
    let baby_trader_meshes = entity_model_textured_meshes(&[baby_trader], &atlas);

    assert_llama_folded_meshes_are_cutout_only(&baby_trader_meshes);
    let baby_trader_mesh = &baby_trader_meshes.cutout;
    let baby_trader_decor = &baby_trader_meshes.armor_cutout;
    assert_eq!(baby_trader_mesh.vertices.len(), baby_bare.vertices.len());
    assert_eq!(baby_trader_decor.vertices.len(), baby_bare.vertices.len());
    assert_vertex_inside_texture(
        baby_trader_decor.vertices[0].uv,
        LLAMA_BODY_TRADER_BABY_TEXTURE_REF,
        &atlas,
    );
}

#[test]
fn llama_carpet_trim_follows_foil_and_explicit_trader_assets_keep_empty_stack_semantics() {
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&[
        LLAMA_CREAMY_TEXTURE_REF,
        LLAMA_CREAMY_BABY_TEXTURE_REF,
        LLAMA_BODY_WHITE_TEXTURE_REF,
        LLAMA_BODY_BLACK_TEXTURE_REF,
        LLAMA_BODY_TRADER_TEXTURE_REF,
        LLAMA_BODY_TRADER_BABY_TEXTURE_REF,
        ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
    ]))
    .unwrap();
    let trim_texture = EntityDynamicTexture { handle: 91_401 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicEntityTextureImage {
        handle: trim_texture.handle,
        size: LLAMA_BODY_WHITE_TEXTURE_REF.size,
        standalone_sampling: None,
        rgba: vec![0xff; 128 * 64 * 4],
    }])
    .unwrap()
    .0;
    let trim = EntityArmorTrim {
        texture: trim_texture,
        decal: false,
    };
    let adult = EntityModelInstance::llama(
        613,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::Llama,
        LlamaVariant::Creamy,
        false,
        false,
    )
    .with(|s| {
        s.kind.llama_mut().body_equipment_asset = Some(test_llama_white_equipment_asset_id());
        s.kind.llama_mut().body_decor_foil = true;
        s.kind.llama_mut().body_decor_trim = Some(trim);
    });

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[adult],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert_llama_folded_meshes_are_cutout_only(&meshes);

    assert_eq!(meshes.armor_entity_glint.vertices.len(), 216);
    assert_eq!(
        meshes.dynamic_player_texture_armor_trim.vertices.len(),
        216,
        "the trim reuses the posed adult LLAMA_DECOR model"
    );

    let adult_trader = EntityModelInstance::llama(
        614,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::TraderLlama,
        LlamaVariant::Creamy,
        false,
        false,
    )
    .with(|s| {
        s.kind.llama_mut().body_equipment_asset = Some(test_llama_trader_equipment_asset_id())
    });
    let trader_equipment = adult_trader;
    let trader_equipment_meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[trader_equipment],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert!(trader_equipment_meshes
        .armor_entity_glint
        .vertices
        .is_empty());
    assert!(trader_equipment_meshes
        .dynamic_player_texture_armor_trim
        .vertices
        .is_empty());

    let baby_trader = EntityModelInstance::llama(
        615,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::TraderLlama,
        LlamaVariant::Creamy,
        true,
        false,
    )
    .with(|s| {
        s.kind.llama_mut().body_equipment_asset = Some(test_llama_trader_baby_equipment_asset_id())
    });
    let baby_trader_meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[baby_trader],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert!(baby_trader_meshes.armor_entity_glint.vertices.is_empty());
    assert!(baby_trader_meshes
        .dynamic_player_texture_armor_trim
        .vertices
        .is_empty());
}

#[test]
fn llama_decor_missing_texture_preserves_other_geometry() {
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
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let bare_meshes = entity_model_textured_meshes(&[adult], &atlas);
    assert_llama_folded_meshes_are_cutout_only(&bare_meshes);

    let white_instance = adult.with(|s| {
        s.kind.llama_mut().body_equipment_asset = Some(test_llama_white_equipment_asset_id())
    });
    let white_meshes = entity_model_textured_meshes(&[white_instance], &atlas);

    assert_llama_folded_meshes_are_cutout_only(&white_meshes);

    assert_eq!(
        white_meshes.cutout.vertices, bare_meshes.cutout.vertices,
        "missing llama_body/white.png suppresses only folded decor geometry"
    );
    assert_eq!(white_meshes.cutout.indices, bare_meshes.cutout.indices);

    assert!(white_meshes
        .cutout
        .vertices
        .iter()
        .all(
            |vertex| vertex.light == white_instance.render_state.shader_light()
                && vertex.overlay == white_instance.render_state.overlay_coords()
        ));

    let baby_bare = EntityModelInstance::llama(
        611,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::Llama,
        LlamaVariant::Creamy,
        true,
        false,
    )
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let baby_bare_meshes = entity_model_textured_meshes(&[baby_bare], &atlas);
    assert_llama_folded_meshes_are_cutout_only(&baby_bare_meshes);
    let baby_trader = EntityModelInstance::llama(
        612,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::TraderLlama,
        LlamaVariant::Creamy,
        true,
        false,
    )
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.kind.llama_mut().body_equipment_asset = Some(test_llama_trader_baby_equipment_asset_id());
    });
    let baby_trader_meshes = entity_model_textured_meshes(&[baby_trader], &atlas);

    assert_llama_folded_meshes_are_cutout_only(&baby_trader_meshes);

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
            |vertex| vertex.light == baby_trader.render_state.shader_light()
                && vertex.overlay == baby_trader.render_state.overlay_coords()
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
    let yawed_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (45.0, 0.0));
    let pitched_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (0.0, -25.0));
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let yawed = entity_model_textured_meshes(&[yawed_instance], &atlas);
    let pitched = entity_model_textured_meshes(&[pitched_instance], &atlas);

    assert_llama_folded_meshes_are_cutout_only(&resting);
    assert_llama_folded_meshes_are_cutout_only(&yawed);
    assert_llama_folded_meshes_are_cutout_only(&pitched);

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
        let still_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.5, 0.0));
        let walking_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let still = entity_model_textured_meshes(&[still_instance], &atlas);
        let walking = entity_model_textured_meshes(&[walking_instance], &atlas);

        assert_llama_folded_meshes_are_cutout_only(&resting);
        assert_llama_folded_meshes_are_cutout_only(&still);
        assert_llama_folded_meshes_are_cutout_only(&walking);

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
    texture_refs_matching(|path| {
        (path.starts_with("textures/entity/llama/")) && !path.ends_with("llama_spit.png")
    })
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
