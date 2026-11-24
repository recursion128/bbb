use super::*;

#[test]
fn special_foil_texture_detection_follows_clock_and_compasses_tag() {
    let root = unique_temp_dir("item-runtime-special-foil");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_item_registry_fixture(&root, &["clock", "compass", "recovery_compass", "spyglass"]);
    for item_id in ["clock", "compass", "recovery_compass", "spyglass"] {
        write_json(
            &assets.join("items").join(format!("{item_id}.json")),
            &format!(
                r#"{{
                        "model": {{ "type": "minecraft:model", "model": "minecraft:item/{item_id}" }}
                    }}"#
            ),
        );
        write_flat_item_model_and_texture(&assets, item_id, &[40, 80, 120, 255]);
    }
    write_json(
        &root
            .join("sources")
            .join(bbb_client_assets::MC_VERSION)
            .join("data")
            .join("minecraft")
            .join("tags")
            .join("item")
            .join("compasses.json"),
        r#"{
                "replace": true,
                "values": ["minecraft:compass"]
            }"#,
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let registry = runtime.registry.as_ref().unwrap();
    let stack = |resource_id: &str| ItemStackSummary {
        item_id: registry.protocol_id(resource_id),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };

    assert!(runtime.item_stack_uses_special_foil_texture(&stack("minecraft:clock")));
    assert!(runtime.item_stack_uses_special_foil_texture(&stack("minecraft:compass")));
    assert!(!runtime.item_stack_uses_special_foil_texture(&stack("minecraft:recovery_compass")));
    assert!(!runtime.item_stack_uses_special_foil_texture(&stack("minecraft:spyglass")));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn item_display_transform_is_retained_per_item() {
    let root = unique_temp_dir("item-display-transform");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_item_registry_fixture_defaults(&root);
    write_json(
        &assets.join("items").join("test_combo.json"),
        r#"{
                "model": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/test_sword"
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("test_combo_alt.json"),
        r#"{
                "model": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/test_sword_alt"
                }
            }"#,
    );
    // An `item/handheld`-style angled third-person transform on the item's own model.
    write_json(
        &assets.join("models").join("item").join("test_sword.json"),
        r##"{
                "display": {
                    "thirdperson_righthand": {
                        "rotation": [0, -90, 55],
                        "translation": [0, 4, 0.5],
                        "scale": [0.85, 0.85, 0.85]
                    }
                },
                "textures": { "layer0": "minecraft:item/test_sword" }
            }"##,
    );
    write_json(
        &assets
            .join("models")
            .join("item")
            .join("test_sword_alt.json"),
        r##"{
                "display": {
                    "thirdperson_righthand": {
                        "rotation": [10, -20, 30],
                        "translation": [1, 8, 2],
                        "scale": [0.4, 0.5, 0.6]
                    }
                },
                "textures": { "layer0": "minecraft:item/test_sword_alt" }
            }"##,
    );
    write_test_rgba_png(
        &assets.join("textures").join("item").join("test_sword.png"),
        1,
        1,
        &[255, 0, 0, 255],
    );
    write_test_rgba_png(
        &assets
            .join("textures")
            .join("item")
            .join("test_sword_alt.png"),
        1,
        1,
        &[0, 255, 0, 255],
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let transform = runtime
        .item_display_transform(0, BlockModelDisplayContext::ThirdPersonRightHand)
        .unwrap();
    // Vanilla pre-multiplies the JSON translation by 1/16 (and clamps); rotation stays in degrees.
    assert_eq!(transform.rotation, [0.0, -90.0, 55.0]);
    assert_eq!(transform.translation, [0.0, 4.0 / 16.0, 0.5 / 16.0]);
    assert_eq!(transform.scale, [0.85, 0.85, 0.85]);
    // A context the model does not override falls back to the identity transform.
    assert_eq!(
        runtime.item_display_transform(0, BlockModelDisplayContext::Gui),
        Some(BlockModelDisplayTransform::default())
    );
    // An unregistered protocol id has no retained transform (caller uses a parent-model default).
    assert_eq!(
        runtime.item_display_transform(999, BlockModelDisplayContext::ThirdPersonRightHand),
        None
    );
    let stack = |component_patch| ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch,
    };
    assert_eq!(
        runtime.item_display_transform_for_stack(
            &stack(DataComponentPatchSummary::default().into()),
            BlockModelDisplayContext::ThirdPersonRightHand,
        ),
        Some(transform)
    );
    let alternate = runtime
        .item_display_transform_for_stack(
            &stack(
                DataComponentPatchSummary {
                    item_model: Some("minecraft:test_combo_alt".to_string()),
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            ),
            BlockModelDisplayContext::ThirdPersonRightHand,
        )
        .unwrap();
    assert_eq!(alternate.rotation, [10.0, -20.0, 30.0]);
    assert_eq!(alternate.translation, [1.0 / 16.0, 8.0 / 16.0, 2.0 / 16.0]);
    assert_eq!(alternate.scale, [0.4, 0.5, 0.6]);
    assert_eq!(
        runtime.item_display_transform_for_stack(
            &stack(
                DataComponentPatchSummary {
                    removed_type_ids: vec![10],
                    ..DataComponentPatchSummary::default()
                }
                .into()
            ),
            BlockModelDisplayContext::ThirdPersonRightHand,
        ),
        None
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn equipment_asset_identity_is_generation_local() {
    let root = unique_temp_dir("item-runtime-humanoid-armor-asset");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_item_registry_fixture(
        &root,
        &[
            "diamond_helmet",
            "diamond_chestplate",
            "elytra",
            "carved_pumpkin",
            "stone",
        ],
    );
    write_item_registry_override_with_profiles(
        &root,
        &[
            "diamond_helmet",
            "diamond_chestplate",
            "elytra",
            "carved_pumpkin",
            "stone",
        ],
        &[
            ("diamond_helmet", "diamond_helmet"),
            ("diamond_chestplate", "diamond_chestplate"),
            ("elytra", "elytra"),
            ("carved_pumpkin", "carved_pumpkin"),
        ],
    );
    write_json(
        &assets.join("equipment").join("diamond.json"),
        r#"{
                "layers": {
                    "humanoid": [
                        { "texture": "minecraft:diamond" }
                    ],
                    "humanoid_leggings": [
                        { "texture": "minecraft:diamond" }
                    ]
                }
            }"#,
    );
    write_json(
        &assets.join("equipment").join("elytra.json"),
        r#"{
                "layers": {
                    "wings": [
                        { "texture": "minecraft:elytra", "use_player_texture": true }
                    ]
                }
            }"#,
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    assert_eq!(
        runtime.equipment_asset_id("minecraft:diamond"),
        Some(EntityEquipmentAssetId::new(0, 0))
    );
    assert_eq!(
        runtime.equipment_asset_id("minecraft:elytra"),
        Some(EntityEquipmentAssetId::new(0, 1))
    );
    assert_eq!(
        runtime.equipment_asset_names(),
        vec![
            "minecraft:diamond".to_string(),
            "minecraft:elytra".to_string()
        ]
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn custom_head_skull_projection_resolves_static_and_profileless_player_skulls() {
    let root = unique_temp_dir("item-runtime-custom-head-skulls");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_item_registry_fixture(
        &root,
        &[
            "skeleton_skull",
            "wither_skeleton_skull",
            "zombie_head",
            "creeper_head",
            "player_head",
            "dragon_head",
            "piglin_head",
            "carved_pumpkin",
        ],
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let registry = runtime.registry.as_ref().unwrap();

    for (resource_id, expected) in [
        (
            "minecraft:skeleton_skull",
            Some(EntityCustomHeadSkull::Skeleton),
        ),
        (
            "minecraft:wither_skeleton_skull",
            Some(EntityCustomHeadSkull::WitherSkeleton),
        ),
        ("minecraft:zombie_head", Some(EntityCustomHeadSkull::Zombie)),
        (
            "minecraft:creeper_head",
            Some(EntityCustomHeadSkull::Creeper),
        ),
        (
            "minecraft:player_head",
            Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Default(
                EntityDefaultPlayerSkin::SlimSteve,
            ))),
        ),
        ("minecraft:dragon_head", Some(EntityCustomHeadSkull::Dragon)),
        ("minecraft:piglin_head", Some(EntityCustomHeadSkull::Piglin)),
        ("minecraft:carved_pumpkin", None),
    ] {
        let protocol_id = registry.protocol_id(resource_id).unwrap();
        let stack = ItemStackSummary {
            item_id: Some(protocol_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        };
        assert_eq!(
            runtime.custom_head_skull_for_stack(&stack),
            expected,
            "{resource_id}"
        );
    }
    assert_eq!(
        runtime.custom_head_skull_for_stack(&ItemStackSummary {
            item_id: Some(999),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        }),
        None
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn custom_head_skull_projection_resolves_profiled_player_head_default_skins() {
    let root = unique_temp_dir("item-runtime-custom-profiled-player-head");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_item_registry_fixture(&root, &["player_head"]);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let registry = runtime.registry.as_ref().unwrap();
    let player_head_id = registry.protocol_id("minecraft:player_head").unwrap();

    let mut profiled = ItemStackSummary {
        item_id: Some(player_head_id),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    profiled
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID);
    assert_eq!(runtime.custom_head_skull_for_stack(&profiled), None);

    profiled.component_patch.profile = Some(ResolvableProfileSummary {
        kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(0)),
        name: None,
        properties: Vec::new(),
        profile_textures: None,
        skin_patch: Default::default(),
    });
    assert_eq!(
        runtime.custom_head_skull_for_stack(&profiled),
        Some(EntityCustomHeadSkull::Player(
            EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
        ))
    );

    let skin_url = "https://textures.minecraft.net/texture/profile-skin";
    let mut dynamic = profiled.clone();
    dynamic.component_patch.profile = Some(ResolvableProfileSummary {
        kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(0)),
        name: None,
        properties: vec![bbb_protocol::packets::GameProfilePropertySummary {
            name: "textures".into(),
            value: "packed-custom-head-textures".into(),
            signature: None,
        }],
        profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
            skin: Some(bbb_protocol::packets::ProfileSkinTextureSummary {
                url: skin_url.to_string(),
                model: bbb_protocol::packets::PlayerModelTypeSummary::Slim,
            }),
            cape: None,
            elytra: None,
        }),
        skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
            body: None,
            cape: None,
            elytra: None,
            model: Some(bbb_protocol::packets::PlayerModelTypeSummary::Wide),
        },
    });
    assert_eq!(
        runtime.custom_head_skull_for_stack(&dynamic),
        Some(EntityCustomHeadSkull::Player(
            EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
        ))
    );

    runtime.enable_profile_network_queues();
    assert_eq!(
        runtime.custom_head_skull_for_stack(&dynamic),
        Some(EntityCustomHeadSkull::Player(
            EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
        ))
    );
    assert_eq!(pump_test_dynamic_textures(&runtime), 1);
    let downloads = drain_all_player_skin_download_results(&runtime);
    assert_eq!(downloads.len(), 1);
    assert_eq!(downloads[0].url, skin_url);
    assert_eq!(
        downloads[0].skin.as_ref().unwrap().handle,
        profile_texture_handle(skin_url)
    );
    assert_eq!(runtime.downloaded_player_skin_count(), 1);
    assert_eq!(
        runtime.custom_head_skull_for_stack(&dynamic),
        Some(EntityCustomHeadSkull::Player(
            EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
        ))
    );
    assert_eq!(pump_test_dynamic_textures(&runtime), 0);

    runtime.mark_profile_skin_resolved(skin_url, 12_345);
    assert_eq!(
        runtime.custom_head_skull_for_stack(&dynamic),
        Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Dynamic(
            EntityDynamicPlayerSkin {
                handle: 12_345,
                fallback: EntityDefaultPlayerSkin::SlimAlex,
                model: EntityPlayerSkinModel::Wide,
                status: EntityDynamicPlayerSkinStatus::Ready,
            }
        )))
    );

    runtime.mark_profile_skin_failed(skin_url);
    assert_eq!(
        runtime.custom_head_skull_for_stack(&dynamic),
        Some(EntityCustomHeadSkull::Player(
            EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
        ))
    );

    let mut resource_patched = dynamic.clone();
    resource_patched
        .component_patch
        .profile
        .as_mut()
        .unwrap()
        .skin_patch
        .body = Some(bbb_protocol::packets::ResourceTextureSummary {
        asset_id: "minecraft:entity/player/custom".to_string(),
        texture_path: "minecraft:textures/entity/player/custom.png".to_string(),
    });
    assert_eq!(
        runtime.custom_head_skull_for_stack(&resource_patched),
        Some(EntityCustomHeadSkull::Player(
            EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
        ))
    );

    let mut patched = profiled.clone();
    patched.component_patch.profile = Some(ResolvableProfileSummary {
        kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(0)),
        name: None,
        properties: Vec::new(),
        profile_textures: None,
        skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
            body: Some(bbb_protocol::packets::ResourceTextureSummary {
                asset_id: "minecraft:entity/player/wide/steve".to_string(),
                texture_path: "minecraft:textures/entity/player/wide/steve.png".to_string(),
            }),
            cape: None,
            elytra: None,
            model: None,
        },
    });
    assert_eq!(
        runtime.custom_head_skull_for_stack(&patched),
        Some(EntityCustomHeadSkull::Player(
            EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::WideSteve)
        ))
    );

    let mut profile_removed = profiled.clone();
    profile_removed
        .component_patch
        .removed_type_ids
        .push(VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID);
    assert_eq!(
        runtime.custom_head_skull_for_stack(&profile_removed),
        Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Default(
            EntityDefaultPlayerSkin::SlimSteve
        )))
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn player_skin_for_profile_resolves_dynamic_owner_before_skin_selection() {
    let uuid = uuid::Uuid::from_u128(0);
    let runtime = NativeItemRuntime::empty_for_test();
    runtime.enable_profile_network_queues();
    assert_eq!(runtime.profile_presentation_revision(), 0);
    let profile = ResolvableProfileSummary {
        kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
        uuid: None,
        name: Some("Alex".to_string()),
        properties: Vec::new(),
        profile_textures: None,
        skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
            body: None,
            cape: None,
            elytra: None,
            model: Some(bbb_protocol::packets::PlayerModelTypeSummary::Wide),
        },
    };

    assert_eq!(
        runtime.player_skin_for_profile(&profile),
        EntityPlayerSkin::ProfiledDefault(default_player_skin_for_profile(&profile))
    );
    assert_eq!(
        runtime.take_pending_profile_resolution_requests(),
        vec![ProfileResolutionKey::Name("Alex".to_string())]
    );
    assert_eq!(
        runtime.apply_profile_resolution_results([ProfileResolutionResult {
            key: ProfileResolutionKey::Name("Alex".to_string()),
            resolved: Some(bbb_protocol::ResolvedGameProfile {
                uuid,
                name: "Alex".into(),
                properties: vec![bbb_protocol::packets::GameProfilePropertySummary {
                    name: "textures".into(),
                    value: "eyJ0aW1lc3RhbXAiOjEsInByb2ZpbGVJZCI6IjAxMjM0NTY3ODlhYmNkZWYwMTIzNDU2Nzg5YWJjZGVmIiwicHJvZmlsZU5hbWUiOiJBbGV4IiwidGV4dHVyZXMiOnsiU0tJTiI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9za2luaGFzaCIsIm1ldGFkYXRhIjp7Im1vZGVsIjoic2xpbSJ9fSwiQ0FQRSI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9jYXBlaGFzaCJ9LCJFTFlUUkEiOnsidXJsIjoiaHR0cHM6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZWx5dHJhaGFzaCJ9fX0=".into(),
                    signature: Some("signature".into()),
                }],
            }),
        }]),
        1
    );
    assert!(runtime.profile_presentation_revision() > 0);
    runtime.player_skin_for_profile(&profile);
    assert_eq!(pump_test_dynamic_textures(&runtime), 3);

    let skin_download = drain_all_player_skin_download_results(&runtime)
        .pop()
        .expect("resolved profile skin download");
    let skin_handle = skin_download.skin.as_ref().unwrap().handle;
    runtime.mark_profile_skin_resolved(&skin_download.url, skin_handle);
    let texture_downloads = drain_all_player_texture_download_results(&runtime);
    assert_eq!(texture_downloads.len(), 2);
    for download in texture_downloads {
        let handle = download.texture.as_ref().unwrap().handle;
        runtime.mark_profile_texture_resolved(download.kind, &download.url, handle);
    }
    assert_eq!(
        runtime.player_skin_for_profile(&profile),
        EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin {
            handle: profile_texture_handle("https://textures.minecraft.net/texture/skinhash"),
            fallback: EntityDefaultPlayerSkin::SlimAlex,
            model: EntityPlayerSkinModel::Wide,
            status: EntityDynamicPlayerSkinStatus::Ready,
        })
    );
    assert!(runtime
        .take_pending_profile_resolution_requests()
        .is_empty());
}

#[test]
fn signed_remote_profile_reaches_three_decoded_pngs_as_one_texture_set() {
    let runtime = NativeItemRuntime::empty_for_test();
    runtime.enable_profile_network_queues();

    let profile_id = uuid::Uuid::from_u128(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef);
    let skin_url = "https://textures.minecraft.net/texture/skinhash";
    let cape_url = "https://textures.minecraft.net/texture/capehash";
    let elytra_url = "https://textures.minecraft.net/texture/elytrahash";
    let profile = bbb_render_types::PlayerInfoProfileState {
        uuid: profile_id,
        name: "Alex".to_string(),
        properties: vec![bbb_protocol::packets::GameProfilePropertySummary {
            name: "textures".into(),
            value: "eyJ0aW1lc3RhbXAiOjEsInByb2ZpbGVJZCI6IjAxMjM0NTY3ODlhYmNkZWYwMTIzNDU2Nzg5YWJjZGVmIiwicHJvZmlsZU5hbWUiOiJBbGV4IiwidGV4dHVyZXMiOnsiU0tJTiI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9za2luaGFzaCIsIm1ldGFkYXRhIjp7Im1vZGVsIjoic2xpbSJ9fSwiQ0FQRSI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9jYXBlaGFzaCJ9LCJFTFlUUkEiOnsidXJsIjoiaHR0cHM6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZWx5dHJhaGFzaCJ9fX0=".into(),
            signature: Some("signature".into()),
        }],
    };

    let blocked = runtime.player_textures_for_player_info(&profile, false, || false);
    assert!(matches!(blocked.skin, EntityPlayerSkin::ProfiledDefault(_)));
    assert_eq!((blocked.cape, blocked.elytra), (None, None));
    assert_eq!(pump_test_dynamic_textures(&runtime), 0);

    let pending = runtime.player_textures_for_player_info(&profile, false, || true);
    assert!(matches!(pending.skin, EntityPlayerSkin::ProfiledDefault(_)));
    assert_eq!((pending.cape, pending.elytra), (None, None));
    assert_eq!(pump_test_dynamic_textures(&runtime), 3);

    let skin = drain_all_player_skin_download_results(&runtime)
        .pop()
        .expect("signed skin result");
    let skin_handle = skin.skin.as_ref().expect("decoded skin PNG").handle;
    runtime.mark_profile_skin_resolved(&skin.url, skin_handle);
    let textures = drain_all_player_texture_download_results(&runtime);
    assert_eq!(textures.len(), 2);
    for texture in &textures {
        let handle = texture
            .texture
            .as_ref()
            .expect("decoded profile PNG")
            .handle;
        runtime.mark_profile_texture_resolved(texture.kind, &texture.url, handle);
    }

    let ready = runtime.player_textures_for_player_info(&profile, false, || true);
    assert!(matches!(
        ready.skin,
        EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin {
            handle,
            model: EntityPlayerSkinModel::Slim,
            status: EntityDynamicPlayerSkinStatus::Ready,
            ..
        }) if handle == profile_texture_handle(skin_url)
    ));
    for (kind, texture, url) in [
        (EntityDynamicPlayerTextureKind::Cape, ready.cape, cape_url),
        (
            EntityDynamicPlayerTextureKind::Elytra,
            ready.elytra,
            elytra_url,
        ),
    ] {
        let texture = texture.expect("complete signed texture set");
        assert_eq!(texture.kind, kind);
        assert_eq!(texture.handle, profile_texture_handle(url));
    }
    assert_eq!(pump_test_dynamic_textures(&runtime), 0);

    assert!(runtime.synchronize_dynamic_texture_session(1, false));
    assert!(runtime.synchronize_dynamic_texture_session(2, true));
    assert_eq!(drain_all_player_skin_download_results(&runtime).len(), 1);
    assert_eq!(drain_all_player_texture_download_results(&runtime).len(), 2);
    assert_eq!(pump_test_dynamic_textures(&runtime), 0);
}

#[test]
fn player_profile_queues_dynamic_cape_and_elytra_texture_downloads() {
    let runtime = NativeItemRuntime::empty_for_test();
    runtime.enable_profile_network_queues();

    let cape_url = "https://textures.minecraft.net/texture/profile-cape";
    let elytra_url = "https://textures.minecraft.net/texture/profile-elytra";
    let profile = ResolvableProfileSummary {
        kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(0)),
        name: None,
        properties: Vec::new(),
        profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
            skin: None,
            cape: Some(bbb_protocol::packets::ProfileTextureSummary {
                url: cape_url.to_string(),
            }),
            elytra: Some(bbb_protocol::packets::ProfileTextureSummary {
                url: elytra_url.to_string(),
            }),
        }),
        skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary::default(),
    };

    assert_eq!(
        runtime.player_skin_for_profile(&profile),
        EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
    );

    assert_eq!(pump_test_dynamic_textures(&runtime), 2);
    let downloads = drain_all_player_texture_download_results(&runtime);
    assert_eq!(downloads.len(), 2);
    let cape = downloads
        .iter()
        .find(|download| download.kind == DynamicPlayerTextureKind::Cape)
        .expect("cape download result");
    assert_eq!(cape.url, cape_url);
    let cape_texture = cape.texture.as_ref().expect("cape texture");
    assert_eq!(cape_texture.handle, profile_texture_handle(cape_url));
    assert_eq!(cape_texture.size, [64, 32]);

    let elytra = downloads
        .iter()
        .find(|download| download.kind == DynamicPlayerTextureKind::Elytra)
        .expect("elytra download result");
    assert_eq!(elytra.url, elytra_url);
    let elytra_texture = elytra.texture.as_ref().expect("elytra texture");
    assert_eq!(elytra_texture.handle, profile_texture_handle(elytra_url));
    assert_eq!(elytra_texture.size, [64, 32]);

    assert_eq!(runtime.downloaded_player_texture_count(), 2);

    runtime.player_skin_for_profile(&profile);
    assert!(runtime
        .drain_dynamic_player_texture_download_results()
        .is_empty());
    assert_eq!(pump_test_dynamic_textures(&runtime), 0);
}

#[test]
fn player_profile_resource_texture_patch_keeps_vanilla_base_downloads() {
    let runtime = NativeItemRuntime::empty_for_test();
    runtime.enable_profile_network_queues();

    let profile = ResolvableProfileSummary {
        kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(0)),
        name: None,
        properties: Vec::new(),
        profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
            skin: None,
            cape: Some(bbb_protocol::packets::ProfileTextureSummary {
                url: "https://textures.minecraft.net/texture/profile-cape".to_string(),
            }),
            elytra: Some(bbb_protocol::packets::ProfileTextureSummary {
                url: "https://textures.minecraft.net/texture/profile-elytra".to_string(),
            }),
        }),
        skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
            body: None,
            cape: Some(bbb_protocol::packets::ResourceTextureSummary {
                asset_id: "minecraft:entity/player/cape/custom".to_string(),
                texture_path: "minecraft:textures/entity/player/cape/custom.png".to_string(),
            }),
            elytra: Some(bbb_protocol::packets::ResourceTextureSummary {
                asset_id: "minecraft:entity/player/elytra/custom".to_string(),
                texture_path: "minecraft:textures/entity/player/elytra/custom.png".to_string(),
            }),
            model: None,
        },
    };

    assert_eq!(
        runtime.player_skin_for_profile(&profile),
        EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
    );
    assert_eq!(pump_test_dynamic_textures(&runtime), 2);
    let downloads = drain_all_player_texture_download_results(&runtime);
    assert_eq!(downloads.len(), 2);
    assert!(downloads.iter().all(|download| {
        download
            .url
            .starts_with("https://textures.minecraft.net/texture/profile-")
    }));
    assert_eq!(runtime.downloaded_player_texture_count(), 2);
}

#[test]
fn player_profile_resource_texture_patch_loads_local_cape_and_elytra_textures() {
    let root = unique_temp_dir("item-runtime-profile-resource-textures");
    let assets = assets_dir(&root);
    let cape_texture_path = "minecraft:textures/entity/player/cape/custom.png";
    let elytra_texture_path = "minecraft:textures/entity/player/elytra/custom.png";
    let cape_rgba = vec![
        10, 0, 0, 255, 20, 0, 0, 255, 30, 0, 0, 255, 40, 0, 0, 255, 50, 0, 0, 255, 60, 0, 0, 255,
        70, 0, 0, 255, 80, 0, 0, 255,
    ];
    let elytra_rgba = vec![
        0, 10, 0, 255, 0, 20, 0, 255, 0, 30, 0, 255, 0, 40, 0, 255, 0, 50, 0, 255, 0, 60, 0, 255,
    ];
    write_test_rgba_png(
        &assets
            .join("textures")
            .join("entity")
            .join("player")
            .join("cape")
            .join("custom.png"),
        4,
        2,
        &cape_rgba,
    );
    write_test_rgba_png(
        &assets
            .join("textures")
            .join("entity")
            .join("player")
            .join("elytra")
            .join("custom.png"),
        2,
        3,
        &elytra_rgba,
    );
    std::fs::write(
        assets
            .join("textures")
            .join("entity")
            .join("player")
            .join("cape")
            .join("custom.png.mcmeta"),
        r#"{"texture":{"blur":true,"clamp":false}}"#,
    )
    .unwrap();
    std::fs::write(
        assets
            .join("textures")
            .join("entity")
            .join("player")
            .join("elytra")
            .join("custom.png.mcmeta"),
        r#"{"texture":{"blur":false,"clamp":true}}"#,
    )
    .unwrap();

    let mut runtime = NativeItemRuntime::empty_for_test();
    runtime.profile_texture_resources =
        PackResourceStack::from_roots([root.join("sources").join(bbb_client_assets::MC_VERSION)]);
    runtime.enable_profile_network_queues();

    let profile = ResolvableProfileSummary {
        kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(0)),
        name: None,
        properties: Vec::new(),
        profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
            skin: None,
            cape: Some(bbb_protocol::packets::ProfileTextureSummary {
                url: "https://textures.minecraft.net/texture/remote-cape".to_string(),
            }),
            elytra: Some(bbb_protocol::packets::ProfileTextureSummary {
                url: "https://textures.minecraft.net/texture/remote-elytra".to_string(),
            }),
        }),
        skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
            body: None,
            cape: Some(ResourceTextureSummary {
                asset_id: "minecraft:entity/player/cape/custom".to_string(),
                texture_path: cape_texture_path.to_string(),
            }),
            elytra: Some(ResourceTextureSummary {
                asset_id: "minecraft:entity/player/elytra/custom".to_string(),
                texture_path: elytra_texture_path.to_string(),
            }),
            model: None,
        },
    };

    let cape_source =
        local_profile_texture_source_id(EntityDynamicPlayerTextureKind::Cape, cape_texture_path);
    let elytra_source = local_profile_texture_source_id(
        EntityDynamicPlayerTextureKind::Elytra,
        elytra_texture_path,
    );
    let cape = runtime
        .player_profile_texture_for_profile(&profile, EntityDynamicPlayerTextureKind::Cape)
        .expect("local cape texture");
    let elytra = runtime
        .player_profile_texture_for_profile(&profile, EntityDynamicPlayerTextureKind::Elytra)
        .expect("local elytra texture");

    assert_eq!(cape.kind, EntityDynamicPlayerTextureKind::Cape);
    assert_eq!(cape.handle, profile_texture_handle(&cape_source));
    assert_eq!(elytra.kind, EntityDynamicPlayerTextureKind::Elytra);
    assert_eq!(elytra.handle, profile_texture_handle(&elytra_source));

    assert_eq!(pump_test_dynamic_textures(&runtime), 2);
    let downloads = drain_all_player_texture_download_results(&runtime);
    assert_eq!(
        downloads
            .iter()
            .filter(|download| download.url.starts_with("https://"))
            .count(),
        2
    );
    let cape_download = downloads
        .iter()
        .find(|download| download.url == cape_source)
        .expect("cape upload");
    assert_eq!(cape_download.url, cape_source);
    let cape_image = cape_download.texture.as_ref().expect("cape image");
    assert_eq!(cape_image.handle, cape.handle);
    assert_eq!(cape_image.size, [4, 2]);
    assert_eq!(
        cape_image.standalone_sampling,
        Some(StandaloneTextureSampling::new(true, false))
    );
    assert_eq!(cape_image.rgba, cape_rgba);

    let elytra_download = downloads
        .iter()
        .find(|download| download.url == elytra_source)
        .expect("elytra upload");
    assert_eq!(elytra_download.url, elytra_source);
    let elytra_image = elytra_download.texture.as_ref().expect("elytra image");
    assert_eq!(elytra_image.handle, elytra.handle);
    assert_eq!(elytra_image.size, [2, 3]);
    assert_eq!(
        elytra_image.standalone_sampling,
        Some(StandaloneTextureSampling::new(false, true))
    );
    assert_eq!(elytra_image.rgba, elytra_rgba);

    assert!(runtime
        .drain_dynamic_player_texture_download_results()
        .is_empty());
    assert_eq!(pump_test_dynamic_textures(&runtime), 0);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn player_profile_resource_texture_patch_failure_does_not_use_remote_or_stale_texture() {
    let root = unique_temp_dir("item-runtime-profile-resource-texture-failure");
    let assets = assets_dir(&root);
    let cape_texture_path = "minecraft:textures/entity/player/cape/missing.png";
    let elytra_texture_path = "minecraft:textures/entity/player/elytra/missing.png";
    let mut runtime = NativeItemRuntime::empty_for_test();
    runtime.profile_texture_resources =
        PackResourceStack::from_roots([root.join("sources").join(bbb_client_assets::MC_VERSION)]);
    runtime.enable_profile_network_queues();
    let profile = ResolvableProfileSummary {
        kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(0)),
        name: None,
        properties: Vec::new(),
        profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
            skin: None,
            cape: Some(bbb_protocol::packets::ProfileTextureSummary {
                url: "https://textures.minecraft.net/texture/remote-cape".to_string(),
            }),
            elytra: Some(bbb_protocol::packets::ProfileTextureSummary {
                url: "https://textures.minecraft.net/texture/remote-elytra".to_string(),
            }),
        }),
        skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
            body: None,
            cape: Some(ResourceTextureSummary {
                asset_id: "minecraft:entity/player/cape/missing".to_string(),
                texture_path: cape_texture_path.to_string(),
            }),
            elytra: Some(ResourceTextureSummary {
                asset_id: "minecraft:entity/player/elytra/missing".to_string(),
                texture_path: elytra_texture_path.to_string(),
            }),
            model: None,
        },
    };

    assert_eq!(
        runtime.player_profile_texture_for_profile(&profile, EntityDynamicPlayerTextureKind::Cape),
        None
    );
    assert_eq!(
        runtime
            .player_profile_texture_for_profile(&profile, EntityDynamicPlayerTextureKind::Elytra),
        None
    );
    assert_eq!(pump_test_dynamic_textures(&runtime), 2);
    let remote_downloads = drain_all_player_texture_download_results(&runtime);
    assert_eq!(remote_downloads.len(), 2);
    assert!(remote_downloads
        .iter()
        .all(|download| download.url.starts_with("https://")));
    assert_eq!(runtime.downloaded_player_texture_count(), 2);

    write_test_rgba_png(
        &assets
            .join("textures")
            .join("entity")
            .join("player")
            .join("cape")
            .join("missing.png"),
        1,
        1,
        &[1, 2, 3, 255],
    );
    write_test_rgba_png(
        &assets
            .join("textures")
            .join("entity")
            .join("player")
            .join("elytra")
            .join("missing.png"),
        1,
        1,
        &[4, 5, 6, 255],
    );

    assert_eq!(
        runtime.player_profile_texture_for_profile(&profile, EntityDynamicPlayerTextureKind::Cape),
        None
    );
    assert_eq!(
        runtime
            .player_profile_texture_for_profile(&profile, EntityDynamicPlayerTextureKind::Elytra),
        None
    );
    assert!(runtime
        .drain_dynamic_player_texture_download_results()
        .is_empty());
    assert_eq!(runtime.downloaded_player_texture_count(), 2);
    assert_eq!(pump_test_dynamic_textures(&runtime), 0);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn player_profile_resource_texture_patch_loads_local_body_skin() {
    let root = unique_temp_dir("item-runtime-profile-resource-body-skin");
    let assets = assets_dir(&root);
    let body_texture_path = "minecraft:textures/entity/player/body/custom.png";
    let mut body_rgba = Vec::with_capacity((64 * 64 * 4) as usize);
    for y in 0..64 {
        for x in 0..64 {
            body_rgba.extend_from_slice(&[x as u8, y as u8, 127, 255]);
        }
    }
    write_test_rgba_png(
        &assets
            .join("textures")
            .join("entity")
            .join("player")
            .join("body")
            .join("custom.png"),
        64,
        64,
        &body_rgba,
    );
    std::fs::write(
        assets
            .join("textures")
            .join("entity")
            .join("player")
            .join("body")
            .join("custom.png.mcmeta"),
        r#"{"texture":{"blur":true,"clamp":true}}"#,
    )
    .unwrap();

    let mut runtime = NativeItemRuntime::empty_for_test();
    runtime.profile_texture_resources =
        PackResourceStack::from_roots([root.join("sources").join(bbb_client_assets::MC_VERSION)]);
    runtime.enable_profile_network_queues();

    let profile = ResolvableProfileSummary {
        kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(0)),
        name: None,
        properties: Vec::new(),
        profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
            skin: Some(bbb_protocol::packets::ProfileSkinTextureSummary {
                url: "https://textures.minecraft.net/texture/remote-skin".to_string(),
                model: bbb_protocol::packets::PlayerModelTypeSummary::Slim,
            }),
            cape: None,
            elytra: None,
        }),
        skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
            body: Some(ResourceTextureSummary {
                asset_id: "minecraft:entity/player/body/custom".to_string(),
                texture_path: body_texture_path.to_string(),
            }),
            cape: None,
            elytra: None,
            model: Some(bbb_protocol::packets::PlayerModelTypeSummary::Wide),
        },
    };
    let body_source = local_player_skin_source_id(body_texture_path);
    let expected_skin = EntityDynamicPlayerSkin {
        handle: profile_texture_handle(&body_source),
        fallback: EntityDefaultPlayerSkin::SlimAlex,
        model: EntityPlayerSkinModel::Wide,
        status: EntityDynamicPlayerSkinStatus::Ready,
    };

    assert_eq!(
        runtime.player_skin_for_profile(&profile),
        EntityPlayerSkin::Dynamic(expected_skin)
    );

    assert_eq!(pump_test_dynamic_textures(&runtime), 1);
    let downloads = drain_all_player_skin_download_results(&runtime);
    assert!(downloads
        .iter()
        .any(|download| download.url.starts_with("https://")));
    let local_download = downloads
        .iter()
        .find(|download| download.url == body_source)
        .expect("local body skin upload");
    let image = local_download.skin.as_ref().expect("local body skin image");
    assert_eq!(image.handle, expected_skin.handle);
    assert_eq!(
        image.standalone_sampling,
        Some(StandaloneTextureSampling::new(true, true))
    );
    assert_eq!(image.rgba, body_rgba);

    assert_eq!(
        runtime.player_skin_for_profile(&profile),
        EntityPlayerSkin::Dynamic(expected_skin)
    );
    let mut other_profile = profile.clone();
    other_profile.uuid = Some(uuid::Uuid::from_u128(1));
    other_profile.skin_patch.model = Some(bbb_protocol::packets::PlayerModelTypeSummary::Slim);
    let other_fallback = default_player_skin_for_profile(&other_profile);
    assert_ne!(other_fallback, expected_skin.fallback);
    assert_eq!(
        runtime.player_skin_for_profile(&other_profile),
        EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin {
            fallback: other_fallback,
            model: EntityPlayerSkinModel::Slim,
            ..expected_skin
        })
    );
    assert!(runtime
        .drain_dynamic_player_skin_download_results()
        .is_empty());
    assert_eq!(pump_test_dynamic_textures(&runtime), 0);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn player_profile_resource_body_patch_failure_does_not_use_remote_or_stale_skin() {
    let root = unique_temp_dir("item-runtime-profile-resource-body-skin-failure");
    let assets = assets_dir(&root);
    let body_texture_path = "minecraft:textures/entity/player/body/missing.png";
    let mut runtime = NativeItemRuntime::empty_for_test();
    runtime.profile_texture_resources =
        PackResourceStack::from_roots([root.join("sources").join(bbb_client_assets::MC_VERSION)]);
    runtime.enable_profile_network_queues();

    let profile = ResolvableProfileSummary {
        kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(0)),
        name: None,
        properties: Vec::new(),
        profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
            skin: Some(bbb_protocol::packets::ProfileSkinTextureSummary {
                url: "https://textures.minecraft.net/texture/remote-skin".to_string(),
                model: bbb_protocol::packets::PlayerModelTypeSummary::Slim,
            }),
            cape: None,
            elytra: None,
        }),
        skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
            body: Some(ResourceTextureSummary {
                asset_id: "minecraft:entity/player/body/missing".to_string(),
                texture_path: body_texture_path.to_string(),
            }),
            cape: None,
            elytra: None,
            model: Some(bbb_protocol::packets::PlayerModelTypeSummary::Wide),
        },
    };

    assert_eq!(
        runtime.player_skin_for_profile(&profile),
        EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
    );
    assert_eq!(pump_test_dynamic_textures(&runtime), 1);
    let remote_downloads = drain_all_player_skin_download_results(&runtime);
    assert_eq!(remote_downloads.len(), 1);
    assert!(remote_downloads[0].url.starts_with("https://"));
    assert_eq!(runtime.downloaded_player_skin_count(), 1);

    let mut body_rgba = Vec::with_capacity((64 * 64 * 4) as usize);
    for y in 0..64 {
        for x in 0..64 {
            body_rgba.extend_from_slice(&[x as u8, y as u8, 191, 255]);
        }
    }
    write_test_rgba_png(
        &assets
            .join("textures")
            .join("entity")
            .join("player")
            .join("body")
            .join("missing.png"),
        64,
        64,
        &body_rgba,
    );

    assert_eq!(
        runtime.player_skin_for_profile(&profile),
        EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
    );
    assert!(runtime
        .drain_dynamic_player_skin_download_results()
        .is_empty());
    assert_eq!(runtime.downloaded_player_skin_count(), 1);
    assert_eq!(pump_test_dynamic_textures(&runtime), 0);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn native_item_runtime_projects_wolf_max_damage() {
    let root = unique_temp_dir("item-runtime-wolf-body-armor");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_item_registry_fixture(&root, &["wolf_armor", "horse_armor"]);
    write_item_registry_override_with_profiles(
        &root,
        &["wolf_armor", "horse_armor"],
        &[("wolf_armor", "wolf_armor")],
    );
    write_json(
        &assets.join("equipment").join("armadillo_scute.json"),
        r#"{
                "layers": {
                    "wolf_body": [
                        { "texture": "minecraft:armadillo_scute" },
                        { "texture": "minecraft:armadillo_scute_overlay", "dyeable": {} }
                    ]
                }
            }"#,
    );
    write_json(
        &assets.join("equipment").join("diamond.json"),
        r#"{
                "layers": {
                    "horse_body": [
                        { "texture": "minecraft:diamond" }
                    ]
                }
            }"#,
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let registry = runtime.registry.as_ref().unwrap();
    let wolf_armor = registry.protocol_id("minecraft:wolf_armor").unwrap();
    let defaults = runtime.default_component_codec_values_by_protocol_id();
    let max_id = component_id::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID;

    assert_eq!(defaults[&wolf_armor][&max_id], serde_json::json!(64));
    assert_eq!(
        defaults
            .values()
            .filter(|values| values.contains_key(&max_id))
            .count(),
        1
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_projects_exact_happy_ghast_harness_assets() {
    let root = unique_temp_dir("item-runtime-happy-ghast-harness");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_item_registry_fixture(
        &root,
        &[
            "white_harness",
            "black_harness",
            "custom_harness",
            "multi_harness",
        ],
    );
    let mut catalog = item_registry_override_value(&[
        "white_harness",
        "black_harness",
        "custom_harness",
        "multi_harness",
    ]);
    for name in [
        "white_harness",
        "black_harness",
        "custom_harness",
        "multi_harness",
    ] {
        catalog["default_component_codec_values"][format!("minecraft:{name}")]
            [component_id::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID.to_string()] =
            serde_json::json!({"slot": "body", "asset_id": format!("minecraft:{name}")});
    }
    write_item_registry_override_value(&root, &catalog);
    for color in ["white", "black"] {
        write_json(
            &assets
                .join("equipment")
                .join(format!("{color}_harness.json")),
            &format!(
                r#"{{
                    "layers": {{
                        "happy_ghast_body": [{{ "texture": "minecraft:{color}_harness" }}]
                    }}
                }}"#
            ),
        );
    }
    write_json(
        &assets.join("equipment").join("custom_harness.json"),
        r#"{
            "layers": {
                "happy_ghast_body": [{ "texture": "minecraft:white_harness" }]
            }
        }"#,
    );
    write_json(
        &assets.join("equipment").join("multi_harness.json"),
        r#"{
            "layers": {
                "happy_ghast_body": [
                    { "texture": "minecraft:white_harness" },
                    { "texture": "minecraft:black_harness" }
                ]
            }
        }"#,
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    for name in [
        "white_harness",
        "black_harness",
        "custom_harness",
        "multi_harness",
    ] {
        assert!(runtime
            .equipment_asset_id(&format!("minecraft:{name}"))
            .is_some());
    }

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_selects_broken_condition_icon_from_stack_damage() {
    let h = FixtureRuntimeHarness::load("item-runtime-broken", |root| {
        write_elytra_damage_condition_fixture(
            root,
            "minecraft:broken",
            "elytra_broken",
            &[120, 80, 40, 255],
        );
    });
    assert_eq!(
        h.runtime
            .registry
            .as_ref()
            .and_then(|registry| registry.protocol_id("minecraft:elytra"))
            .and_then(|item_id| h.defaults.item_max_damage_for_protocol_id(item_id)),
        Some(432)
    );
    assert_eq!(h.runtime.icon_texture_count(), 1);
    assert_eq!(
        h.runtime.icon_texture_index_for_protocol_id(0),
        Some(h.runtime.texture_index("minecraft:item/elytra"))
    );

    let dmg = component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID;
    let unbreakable = component_id::VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID;
    h.assert_profiled_models(
        0,
        [
            (patch_with(&[dmg], |p| p.damage = Some(430)), "elytra"),
            (
                patch_with(&[dmg], |p| p.damage = Some(431)),
                "elytra_broken",
            ),
            (
                patch_with(&[dmg, unbreakable], |p| {
                    p.damage = Some(431);
                    p.unbreakable = true;
                }),
                "elytra",
            ),
            (
                patch_with(&[dmg], |p| {
                    p.damage = Some(431);
                    p.removed_type_ids = vec![2];
                }),
                "elytra",
            ),
            (patch_removed(&[3]), "elytra"),
        ],
    );
}

#[test]
fn native_item_runtime_selects_damaged_condition_icon_from_stack_damage() {
    let h = FixtureRuntimeHarness::load("item-runtime-damaged", |root| {
        write_elytra_damage_condition_fixture(
            root,
            "minecraft:damaged",
            "elytra_damaged",
            &[80, 120, 40, 255],
        );
    });
    assert_eq!(
        h.runtime.icon_texture_index_for_protocol_id(0),
        Some(h.runtime.texture_index("minecraft:item/elytra"))
    );

    let dmg = component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID;
    let unbreakable = component_id::VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID;
    h.assert_profiled_models(
        0,
        [
            (DataComponentPatchSummary::default(), "elytra"),
            (patch_with(&[dmg], |p| p.damage = Some(1)), "elytra_damaged"),
            (
                patch_with(&[dmg, unbreakable], |p| {
                    p.damage = Some(1);
                    p.unbreakable = true;
                }),
                "elytra",
            ),
            (
                patch_with(&[dmg], |p| {
                    p.damage = Some(1);
                    p.removed_type_ids = vec![3];
                }),
                "elytra",
            ),
        ],
    );
}

#[test]
fn native_item_runtime_selects_has_component_icon_from_dyed_color() {
    let h = FixtureRuntimeHarness::load(
        "item-runtime-has-dyed-color",
        write_wolf_armor_has_component_fixture,
    );
    assert_eq!(
        h.runtime.icon_texture_index_for_protocol_id(0),
        Some(h.runtime.texture_index("minecraft:item/wolf_armor"))
    );

    let gui = ItemModelContext::new(BlockModelDisplayContext::Gui);
    for (patch, expected_model, expected_tint) in [
        (
            DataComponentPatchSummary::default(),
            "wolf_armor",
            ITEM_TINT_WHITE,
        ),
        (
            patch_with(&[44], |p| p.dyed_color = Some(0x33_66_99)),
            "wolf_armor_dyed",
            rgb_i32_tint(0x33_66_99),
        ),
        (
            patch_with(&[44], |p| {
                p.removed_type_ids = vec![44];
                p.dyed_color = Some(0x33_66_99);
            }),
            "wolf_armor",
            ITEM_TINT_WHITE,
        ),
    ] {
        let icon = h.flat_icon_with_context(0, patch, &gui);
        assert_eq!(icon.layers[0].uv, h.uv(expected_model));
        assert_eq!(icon.layers[0].tint, expected_tint);
    }
}

#[test]
fn native_item_runtime_selects_has_component_icon_from_lodestone_tracker() {
    let h = FixtureRuntimeHarness::load(
        "item-runtime-has-lodestone",
        write_compass_has_component_fixture,
    );
    assert_eq!(
        h.runtime.icon_texture_index_for_protocol_id(0),
        Some(h.runtime.texture_index("minecraft:item/compass"))
    );

    h.assert_gui_models(
        0,
        [
            (DataComponentPatchSummary::default(), "compass"),
            (patch_with(&[67], |_| {}), "compass_lodestone"),
            (
                patch_with(&[67], |p| p.removed_type_ids = vec![67]),
                "compass",
            ),
        ],
    );
}

#[test]
fn native_item_runtime_selects_has_component_defaults_and_nondefault_patches() {
    let h = FixtureRuntimeHarness::load(
        "item-runtime-has-component-defaults",
        write_default_has_component_fixture,
    );
    // Vanilla `HasComponent.get(ignoreDefault=false)` calls
    // `ItemStack.has`, so common prototype components count as present.
    h.assert_gui_models(
        0,
        [
            (
                DataComponentPatchSummary::default(),
                "has_max_stack_present",
            ),
            (patch_removed(&[1]), "has_max_stack_absent"),
        ],
    );
    // With `ignore_default=true`, vanilla checks hasNonDefault: both added
    // and removed patches are non-default, while the untouched prototype is
    // not.
    h.assert_gui_models(
        1,
        [
            (
                DataComponentPatchSummary::default(),
                "has_max_stack_unpatched",
            ),
            (
                patch_with(&[1], |p| p.max_stack_size = Some(16)),
                "has_max_stack_patched",
            ),
            (patch_removed(&[1]), "has_max_stack_patched"),
        ],
    );
    // `rarity=common` is also in vanilla `COMMON_ITEM_COMPONENTS`.
    for (item_id, removed_id, present_model, absent_model) in [
        (2, 12, "has_rarity_present", "has_rarity_absent"),
        (3, 13, "has_enchantments_present", "has_enchantments_absent"),
        (
            4,
            42,
            "has_stored_enchantments_present",
            "has_stored_enchantments_absent",
        ),
    ] {
        h.assert_gui_models(
            item_id,
            [
                (DataComponentPatchSummary::default(), present_model),
                (patch_removed(&[removed_id]), absent_model),
            ],
        );
    }
}
