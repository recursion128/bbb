use super::*;

#[test]
fn item_uv_rect_matches_exact_content_edges_without_half_texel_inset() {
    let textures = ItemTextureState::from_images(vec![SpriteImage::new(
        "minecraft:item/test",
        16,
        16,
        vec![255; 16 * 16 * 4],
    )
    .unwrap()])
    .unwrap();
    let sprite = &textures.atlas.layout.sprites[0];
    let uv = item_uv_rect(&textures.atlas.layout, sprite);
    let w = textures.atlas.layout.width as f32;
    let h = textures.atlas.layout.height as f32;
    assert_eq!(
        uv.min,
        [sprite.content.x as f32 / w, sprite.content.y as f32 / h]
    );
    assert_eq!(
        uv.max,
        [
            (sprite.content.x + sprite.content.width) as f32 / w,
            (sprite.content.y + sprite.content.height) as f32 / h,
        ]
    );
}

#[test]
fn world_text_font_direction_tracks_the_selected_language_owner() {
    let mut fonts = Some(test_map_text_fonts());
    assert!(!fonts.as_ref().unwrap().default_right_to_left());

    set_map_text_direction(&mut fonts, true);
    assert!(fonts.as_ref().unwrap().default_right_to_left());

    set_map_text_direction(&mut fonts, false);
    assert!(!fonts.as_ref().unwrap().default_right_to_left());
}

#[test]
fn client_item_swap_properties_follow_effective_stack_model_and_projection() {
    let mut runtime = NativeItemRuntime::empty_for_test().with_vanilla_item_registry_for_test();
    runtime.client_item_properties.insert(
        "custom:no_swap_animation".to_string(),
        ClientItemProperties {
            hand_animation_on_swap: false,
            oversized_in_gui: false,
            swap_animation_scale: 0.625,
        },
    );
    let apple_id = runtime.item_protocol_id("minecraft:apple").unwrap();
    let stack = |component_patch| ItemStackSummary {
        item_id: Some(apple_id),
        count: 1,
        component_patch,
    };

    let default_stack = stack(DataComponentPatchSummary::default().into());
    assert_eq!(
        runtime.item_model_id_for_stack(&default_stack),
        Some("minecraft:apple")
    );
    assert!(runtime.hand_animation_on_swap_for_stack(&default_stack));
    assert_eq!(runtime.swap_animation_scale_for_stack(&default_stack), 1.0);

    let override_stack = stack(
        DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID],
            item_model: Some("custom:no_swap_animation".to_string()),
            ..DataComponentPatchSummary::default()
        }
        .into(),
    );
    assert_eq!(
        runtime.item_model_id_for_stack(&override_stack),
        Some("custom:no_swap_animation")
    );
    assert!(!runtime.hand_animation_on_swap_for_stack(&override_stack));
    assert_eq!(
        runtime.swap_animation_scale_for_stack(&override_stack),
        0.625
    );

    let removed_model = stack(
        DataComponentPatchSummary {
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID],
            ..DataComponentPatchSummary::default()
        }
        .into(),
    );
    assert_eq!(runtime.item_model_id_for_stack(&removed_model), None);
    assert!(runtime.hand_animation_on_swap_for_stack(&removed_model));
    assert_eq!(runtime.swap_animation_scale_for_stack(&removed_model), 1.0);
}

#[test]
fn client_item_swap_match_ignores_only_damage_values_not_membership_or_count() {
    let runtime = NativeItemRuntime::empty_for_test().with_vanilla_item_registry_for_test();
    let sword_id = runtime.item_protocol_id("minecraft:diamond_sword").unwrap();
    let stack = |count, component_patch| ItemStackSummary {
        item_id: Some(sword_id),
        count,
        component_patch,
    };
    let damaged = |damage| {
        stack(
            1,
            DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID],
                damage: Some(damage),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        )
    };

    assert!(runtime.stacks_match_ignoring_swap_animation(&damaged(2), &damaged(17)));
    assert!(!runtime
        .stacks_match_ignoring_swap_animation(&damaged(2), &stack(2, damaged(17).component_patch)));
    assert!(!runtime.stacks_match_ignoring_swap_animation(
        &stack(1, DataComponentPatchSummary::default().into()),
        &stack(
            1,
            DataComponentPatchSummary {
                removed_type_ids: vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID],
                ..DataComponentPatchSummary::default()
            }
            .into()
        )
    ));
}

#[test]
fn minimum_attack_charge_projects_vanilla_spear_defaults_by_protocol_id() {
    let runtime = NativeItemRuntime::empty_for_test().with_vanilla_item_registry_for_test();
    let wooden_spear_id = runtime.item_protocol_id("minecraft:wooden_spear").unwrap();
    let diamond_sword_id = runtime.item_protocol_id("minecraft:diamond_sword").unwrap();
    let defaults = runtime.default_component_codec_values_by_protocol_id();
    let charge_id =
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID;

    assert_eq!(
        defaults[&wooden_spear_id][&charge_id],
        serde_json::json!(1.0)
    );
    assert!(!defaults[&diamond_sword_id].contains_key(&charge_id));
    assert_eq!(
        defaults
            .values()
            .filter(|values| values.contains_key(&charge_id))
            .count(),
        7
    );
}

#[test]
fn dynamic_texture_session_edge_retains_cpu_image_and_redelivers_without_fetch() {
    let runtime = NativeItemRuntime::empty_for_test();
    assert_eq!(runtime.profile_presentation_revision(), 0);
    runtime.enable_profile_network_queues();
    let url = "https://textures.minecraft.net/texture/session-skin";
    let profile = ResolvableProfileSummary {
        kind: ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(41)),
        name: None,
        properties: vec![bbb_protocol::packets::GameProfilePropertySummary {
            name: "textures".into(),
            value: "packed-profile-skin".into(),
            signature: None,
        }],
        profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
            skin: Some(bbb_protocol::packets::ProfileSkinTextureSummary {
                url: url.to_string(),
                model: bbb_protocol::packets::PlayerModelTypeSummary::Wide,
            }),
            cape: None,
            elytra: None,
        }),
        skin_patch: Default::default(),
    };

    assert!(runtime.synchronize_dynamic_texture_session(3, true));
    assert_eq!(runtime.profile_presentation_revision(), 1);
    assert!(matches!(
        runtime.player_skin_for_profile(&profile),
        EntityPlayerSkin::ProfiledDefault(_)
    ));
    assert_eq!(pump_test_dynamic_textures(&runtime), 1);
    let first_download = drain_all_player_skin_download_results(&runtime)
        .pop()
        .expect("initial skin delivery");
    let handle = first_download.skin.as_ref().unwrap().handle;
    runtime.mark_profile_skin_resolved(url, handle);
    assert_eq!(runtime.profile_presentation_revision(), 2);
    assert!(matches!(
        runtime.player_skin_for_profile(&profile),
        EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin {
            handle: resolved,
            status: EntityDynamicPlayerSkinStatus::Ready,
            ..
        }) if resolved == handle
    ));

    assert!(!runtime.synchronize_dynamic_texture_session(3, true));
    assert_eq!(runtime.profile_presentation_revision(), 2);
    assert!(matches!(
        runtime.player_skin_for_profile(&profile),
        EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin {
            handle: resolved,
            status: EntityDynamicPlayerSkinStatus::Ready,
            ..
        }) if resolved == handle
    ));

    assert!(runtime.synchronize_dynamic_texture_session(3, false));
    assert_eq!(runtime.profile_presentation_revision(), 3);
    assert!(matches!(
        runtime.player_skin_for_profile(&profile),
        EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin {
            handle: resolved,
            status: EntityDynamicPlayerSkinStatus::Ready,
            ..
        }) if resolved == handle
    ));
    let observation = runtime.dynamic_texture_cache_observation();
    assert!(!observation.accepting_requests);
    assert_eq!(observation.profile_sets, 1);
    assert_eq!(observation.profile_assets, 1);
    assert!(observation.profile_key_bytes > 0);
    assert_eq!(observation.entries, 1);
    assert!(observation.resident_bytes > 0);
    assert_eq!(pump_test_dynamic_textures(&runtime), 0);
    assert!(runtime.synchronize_dynamic_texture_session(4, true));
    assert_eq!(runtime.profile_presentation_revision(), 4);
    let redelivery = runtime.drain_dynamic_player_skin_download_results();
    assert_eq!(redelivery.len(), 1);
    assert_eq!(redelivery[0].url, url);
    assert_eq!(redelivery[0].skin.as_ref().unwrap().handle, handle);
    assert_eq!(pump_test_dynamic_textures(&runtime), 0);
}

#[test]
fn successful_resource_generation_adopts_one_process_texture_owner_and_redelivers() {
    let mut previous = NativeItemRuntime::empty_for_test();
    previous.enable_profile_network_queues();
    previous.synchronize_dynamic_texture_session(5, true);
    let url = "https://textures.minecraft.net/texture/resource-adoption";
    let profile = ResolvableProfileSummary {
        kind: ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(42)),
        name: None,
        properties: vec![bbb_protocol::packets::GameProfilePropertySummary {
            name: "textures".into(),
            value: "packed-resource-adoption".into(),
            signature: None,
        }],
        profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
            skin: Some(bbb_protocol::packets::ProfileSkinTextureSummary {
                url: url.to_string(),
                model: bbb_protocol::packets::PlayerModelTypeSummary::Wide,
            }),
            cape: None,
            elytra: None,
        }),
        skin_patch: Default::default(),
    };
    previous.player_skin_for_profile(&profile);
    assert_eq!(pump_test_dynamic_textures(&previous), 1);
    let initial = drain_all_player_skin_download_results(&previous)
        .pop()
        .expect("initial process texture delivery");
    let handle = initial.skin.as_ref().unwrap().handle;
    previous.mark_profile_skin_resolved(url, handle);

    let mut prepared = NativeItemRuntime::empty_for_test();
    prepared.adopt_profile_texture_runtime(&mut previous);

    assert_eq!(previous.dynamic_texture_cache_observation().entries, 0);
    assert!(matches!(
        prepared.player_skin_for_profile(&profile),
        EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin {
            handle: resolved,
            status: EntityDynamicPlayerSkinStatus::Ready,
            ..
        }) if resolved == handle
    ));
    let redelivery = prepared.drain_dynamic_player_skin_download_results();
    assert_eq!(redelivery.len(), 1);
    assert_eq!(redelivery[0].url, url);
    assert_eq!(redelivery[0].skin.as_ref().unwrap().handle, handle);
    assert_eq!(pump_test_dynamic_textures(&prepared), 0);
}

#[test]
fn profile_presentation_revision_tracks_skin_failure_after_success() {
    let runtime = NativeItemRuntime::empty_for_test();
    let url = "https://textures.minecraft.net/texture/revision-skin";
    let profile = ResolvableProfileSummary {
        kind: ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(43)),
        name: None,
        properties: Vec::new(),
        profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
            skin: Some(bbb_protocol::packets::ProfileSkinTextureSummary {
                url: url.to_string(),
                model: bbb_protocol::packets::PlayerModelTypeSummary::Wide,
            }),
            cape: None,
            elytra: None,
        }),
        skin_patch: Default::default(),
    };
    runtime.player_skin_for_profile(&profile);
    runtime.mark_profile_skin_resolved(url, 7);
    assert_eq!(runtime.profile_presentation_revision(), 1);
    runtime.mark_profile_skin_resolved(url, 7);
    assert_eq!(runtime.profile_presentation_revision(), 1);
    runtime.mark_profile_skin_failed(url);
    assert_eq!(runtime.profile_presentation_revision(), 2);
}

#[test]
fn vanilla_item_selector_requires_canonical_namespaced_resource_id() {
    assert!(item_types::vanilla_item_resource_id_matches_type(
        "minecraft:bow",
        item_types::VANILLA_ITEM_BOW_ID
    ));
    assert!(!item_types::vanilla_item_resource_id_matches_type(
        "bow",
        item_types::VANILLA_ITEM_BOW_ID
    ));
    assert!(!item_types::vanilla_item_resource_id_matches_type(
        "example:bow",
        item_types::VANILLA_ITEM_BOW_ID
    ));
}

#[test]
fn item_texture_state_indexes_textures_and_uses_missing_fallback() {
    let missing = SpriteImage::new("minecraft:missingno", 1, 1, vec![0, 0, 0, 255]).unwrap();
    let apple = SpriteImage::new("minecraft:item/apple", 1, 1, vec![255, 0, 0, 127]).unwrap();

    let state = ItemTextureState::from_images(vec![missing, apple]).unwrap();

    assert_eq!(state.texture_count(), 2);
    assert_ne!(
        state.texture_index("minecraft:item/apple"),
        state.texture_index("minecraft:missingno")
    );
    assert_eq!(
        state.texture_index("custom:item/missing"),
        state.texture_index(MISSING_TEXTURE_ID)
    );
    let sprite_uvs = state.sprite_uvs();
    assert_eq!(sprite_uvs.len(), state.texture_count());
    assert!(sprite_uvs
        .iter()
        .any(|sprite| sprite.id == "minecraft:item/apple" && sprite.has_translucent));
    assert!(sprite_uvs
        .iter()
        .any(|sprite| sprite.id == MISSING_TEXTURE_ID && !sprite.has_translucent));

    let apple_index = state.texture_index("minecraft:item/apple");
    let first_mask = state.alpha_mask(apple_index).unwrap();
    let second_mask = state.alpha_mask(apple_index).unwrap();
    assert_eq!(first_mask, second_mask);
    assert!(!first_mask.is_transparent(0, 0));
    assert_eq!(
        state.texture_asset_id(apple_index).as_deref(),
        Some("minecraft:item/apple")
    );
}

#[test]
fn item_texture_animation_patches_only_the_selected_sprite_region() {
    let missing = SpriteImage::new("minecraft:missingno", 1, 1, vec![0, 0, 0, 255]).unwrap();
    let mut animated =
        SpriteImage::new("custom:item/animated", 1, 1, vec![255, 0, 0, 255]).unwrap();
    animated.animation = Some(bbb_client_assets::SpriteAnimation {
        frame_count: 2,
        default_frame_time: 1,
        interpolate: false,
        frames: vec![
            bbb_client_assets::SpriteAnimationFrame { index: 0, time: 1 },
            bbb_client_assets::SpriteAnimationFrame { index: 1, time: 1 },
        ],
    });
    animated.animation_frames_rgba = vec![vec![255, 0, 0, 255], vec![0, 255, 0, 255]];

    let images = vec![missing, animated];
    let expected = AtlasPacker::new(ITEM_ATLAS_MAX_WIDTH, 1)
        .unwrap()
        .stitch_animation_frame(&images, 1)
        .unwrap()
        .rgba;
    let state = ItemTextureState::from_images(images).unwrap();
    assert!(state.has_animation());
    let patches = state.animation_atlas_patches(0, 1).unwrap().unwrap();
    let mut patched = state.atlas.rgba.clone();
    for patch in &patches {
        assert_eq!(patch.level, 0);
        for row in 0..patch.rect.height {
            let source_start = (row * patch.rect.width * 4) as usize;
            let destination_start =
                (((patch.rect.y + row) * state.atlas.layout.width + patch.rect.x) * 4) as usize;
            let byte_count = (patch.rect.width * 4) as usize;
            patched[destination_start..destination_start + byte_count]
                .copy_from_slice(&patch.rgba[source_start..source_start + byte_count]);
        }
    }
    assert_eq!(patched, expected);

    let sprite = &state.atlas.layout.sprites[state.texture_index("custom:item/animated") as usize];
    let offset = ((sprite.content.y * state.atlas.layout.width + sprite.content.x) * 4) as usize;
    assert_eq!(&patched[offset..offset + 4], &[0, 255, 0, 255]);
}

#[test]
fn item_texture_alpha_mask_unions_unique_animation_frames() {
    let missing = SpriteImage::new("minecraft:missingno", 1, 1, vec![0, 0, 0, 255]).unwrap();
    let mut animated =
        SpriteImage::new("custom:item/animated", 2, 1, vec![0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
    animated.animation = Some(bbb_client_assets::SpriteAnimation {
        frame_count: 2,
        default_frame_time: 1,
        interpolate: false,
        frames: vec![
            bbb_client_assets::SpriteAnimationFrame { index: 0, time: 1 },
            bbb_client_assets::SpriteAnimationFrame { index: 1, time: 1 },
        ],
    });
    animated.animation_frames_rgba = vec![
        vec![255, 255, 255, 255, 0, 0, 0, 0],
        vec![0, 0, 0, 0, 255, 255, 255, 255],
    ];

    let state = ItemTextureState::from_images(vec![missing, animated]).unwrap();
    let mask = state
        .alpha_mask(state.texture_index("custom:item/animated"))
        .unwrap();
    assert!(!mask.is_transparent(0, 0));
    assert!(!mask.is_transparent(1, 0));
}

#[test]
fn item_tint_sources_use_stack_component_colors_when_available() {
    let patch = DataComponentPatchSummary {
        custom_model_data_colors: vec![0x01_02_03, 0x04_05_06],
        dyed_color: Some(0x07_08_09),
        map_color: Some(0x0a_0b_0c),
        potion_contents: bbb_protocol::packets::PotionContentsSummary {
            custom_color: Some(0x0d_0e_0f),
            ..Default::default()
        },
        firework_explosion: Some(bbb_protocol::packets::FireworkExplosionSummary {
            colors: vec![0x10_20_30, 0x20_40_60],
            ..Default::default()
        }),
        ..DataComponentPatchSummary::default()
    };

    assert_eq!(
        item_tint_source_color(
            &ItemTintSource::CustomModelData {
                index: 1,
                default_color: 0xff_00_ff,
            },
            Some(&patch),
            None,
        ),
        rgb_i32_tint(0x04_05_06)
    );
    assert_eq!(
        item_tint_source_color(
            &ItemTintSource::Dye {
                default_color: 0xff_00_ff,
            },
            Some(&patch),
            None,
        ),
        rgb_i32_tint(0x07_08_09)
    );
    assert_eq!(
        item_tint_source_color(
            &ItemTintSource::MapColor {
                default_color: 0xff_00_ff,
            },
            Some(&patch),
            None,
        ),
        rgb_i32_tint(0x0a_0b_0c)
    );
    assert_eq!(
        item_tint_source_color(
            &ItemTintSource::Potion {
                default_color: 0xff_00_ff,
            },
            Some(&patch),
            None,
        ),
        rgb_i32_tint(0x0d_0e_0f)
    );
    assert_eq!(
        item_tint_source_color(
            &ItemTintSource::Firework {
                default_color: 0xff_00_ff,
            },
            Some(&patch),
            None,
        ),
        rgb_i32_tint(0x18_30_48)
    );
    assert_eq!(
        item_tint_source_color(
            &ItemTintSource::CustomModelData {
                index: 2,
                default_color: 0xff_00_ff,
            },
            Some(&patch),
            None,
        ),
        rgb_i32_tint(0xff_00_ff)
    );
    assert_eq!(
        item_tint_source_color(
            &ItemTintSource::Firework {
                default_color: 0xff_00_ff,
            },
            Some(&DataComponentPatchSummary::default()),
            None,
        ),
        rgb_i32_tint(0xff_00_ff)
    );
}

#[test]
fn potion_item_tint_matches_vanilla_visible_effect_weighting_and_fallbacks() {
    let tint = ItemTintSource::Potion {
        default_color: 0x12_34_56,
    };
    let custom_effect = |effect_id, amplifier, show_particles| MobEffectInstanceSummary {
        effect_id,
        amplifier,
        duration: 200,
        ambient: false,
        show_particles,
        show_icon: true,
        hidden_effect: None,
    };

    assert_eq!(
        item_tint_source_color(
            &tint,
            Some(&DataComponentPatchSummary {
                potion_contents: bbb_protocol::packets::PotionContentsSummary {
                    custom_color: Some(0xab_cd_ef),
                    potion_id: Some(13),
                    custom_effects: vec![custom_effect(VANILLA_MOB_EFFECT_SLOWNESS_ID, 1, true,)],
                    ..Default::default()
                },
                ..DataComponentPatchSummary::default()
            }),
            None,
        ),
        rgb_i32_tint(0xab_cd_ef)
    );
    assert_eq!(
        item_tint_source_color(
            &tint,
            Some(&DataComponentPatchSummary {
                potion_contents: bbb_protocol::packets::PotionContentsSummary {
                    potion_id: Some(13),
                    ..Default::default()
                },
                ..DataComponentPatchSummary::default()
            }),
            None,
        ),
        rgb_i32_tint(0x33_eb_ff)
    );
    assert_eq!(
        item_tint_source_color(
            &tint,
            Some(&DataComponentPatchSummary {
                potion_contents: bbb_protocol::packets::PotionContentsSummary {
                    potion_id: Some(13),
                    custom_effects: vec![custom_effect(VANILLA_MOB_EFFECT_SLOWNESS_ID, 1, true,)],
                    ..Default::default()
                },
                ..DataComponentPatchSummary::default()
            }),
            None,
        ),
        rgb_i32_tint(0x6d_c3_ea)
    );
    assert_eq!(
        item_tint_source_color(
            &tint,
            Some(&DataComponentPatchSummary {
                potion_contents: bbb_protocol::packets::PotionContentsSummary {
                    custom_effects: vec![custom_effect(VANILLA_MOB_EFFECT_SLOWNESS_ID, 1, false,)],
                    ..Default::default()
                },
                ..DataComponentPatchSummary::default()
            }),
            None,
        ),
        rgb_i32_tint(0x12_34_56)
    );
    assert_eq!(
        item_tint_source_color(&tint, Some(&DataComponentPatchSummary::default()), None),
        rgb_i32_tint(0x12_34_56)
    );
}

#[test]
fn team_item_tint_uses_living_owner_scoreboard_color_or_default() {
    let tint = ItemTintSource::Team {
        default_color: 0x12_34_56,
    };

    assert_eq!(
        item_tint_source_color(&tint, None, Some(0xab_cd_ef)),
        rgb_i32_tint(0xab_cd_ef)
    );
    assert_eq!(
        item_tint_source_color(&tint, None, None),
        rgb_i32_tint(0x12_34_56)
    );
}

#[test]
fn localized_item_name_prefers_item_key_then_block_key() {
    let language = LanguageCatalog::from_json_bytes(
        br#"{
                "item.minecraft.redstone": "Redstone Dust",
                "block.minecraft.stone": "Stone"
            }"#,
    )
    .unwrap();

    assert_eq!(
        localized_item_name(&language, "minecraft:redstone"),
        "Redstone Dust"
    );
    assert_eq!(localized_item_name(&language, "minecraft:stone"), "Stone");
    assert_eq!(
        localized_item_name(&language, "minecraft:missing_item"),
        "item.minecraft.missing_item"
    );
    assert_eq!(
        description_key("item", "custom:tools/hammer"),
        "item.custom.tools.hammer"
    );
}

#[test]
fn localized_text_presence_distinguishes_identity_empty_missing_and_replacement() {
    let mut runtime = NativeItemRuntime::empty_for_test();
    runtime.language = LanguageCatalog::from_json_bytes(
        br#"{"test.identity":"test.identity","test.empty":"","test.changed":"first"}"#,
    )
    .unwrap();
    assert_eq!(
        runtime.localized_text_if_present("test.identity"),
        Some("test.identity")
    );
    assert_eq!(runtime.localized_text_if_present("test.empty"), Some(""));
    assert_eq!(runtime.localized_text_if_present("test.missing"), None);
    assert_eq!(runtime.localized_text("test.missing"), "test.missing");
    assert_eq!(
        runtime.localized_text_if_present("test.changed"),
        Some("first")
    );
    runtime.language = LanguageCatalog::from_json_bytes(br#"{"test.changed":"second"}"#).unwrap();
    assert_eq!(
        runtime.localized_text_if_present("test.changed"),
        Some("second")
    );
    assert_eq!(runtime.localized_text_if_present("test.identity"), None);
}
