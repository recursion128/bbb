use super::*;
use crate::entity_models::equipment::ResolvedEntityEquipmentAssetCatalog;
use crate::DynamicEntityTextureImage;
use std::collections::BTreeMap;

const GENERATION: u64 = 71;

fn empty_static_atlas() -> EntityModelTextureAtlasLayout {
    EntityModelTextureAtlasLayout {
        width: 1,
        height: 1,
        entries: Vec::new(),
    }
}

fn owned_entry(texture: EntityModelOwnedTextureRef) -> EntityModelOwnedTextureAtlasEntry {
    EntityModelOwnedTextureAtlasEntry {
        texture,
        uv: EntityModelUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
    }
}

fn single_asset_catalog(
    layers: BTreeMap<EntityEquipmentLayerType, Vec<EntityEquipmentAssetLayer>>,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
) -> ResolvedEntityEquipmentAssetCatalog {
    EntityEquipmentAssetCatalog::new(GENERATION, vec![EntityEquipmentAsset { layers }])
        .resolve(owned_entries)
        .unwrap()
}

fn owned_layer(texture: EntityModelOwnedTextureRef) -> EntityEquipmentAssetLayer {
    EntityEquipmentAssetLayer {
        texture: EntityEquipmentAssetLayerTexture::Owned(texture),
        dyeable: None,
        use_player_texture: false,
    }
}

fn render(
    instance: EntityModelInstance,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) -> EntityModelTexturedMeshes {
    entity_model_textured_meshes_with_equipment_for_camera(
        &[instance],
        &empty_static_atlas(),
        owned_entries,
        equipment_assets,
        None,
        dynamic_texture_atlas,
        None,
        None,
    )
}

#[test]
fn production_equipment_rendering_has_no_static_compatibility_resolver() {
    let production_sources = [
        ("equipment.rs", include_str!("../equipment.rs")),
        ("textured.rs", include_str!("../textured.rs")),
        ("gui_preview.rs", include_str!("../gui_preview.rs")),
        (
            "model_layers/armor.rs",
            include_str!("../model_layers/armor.rs"),
        ),
        (
            "model_layers/textures.rs",
            include_str!("../model_layers/textures.rs"),
        ),
        (
            "model_layers/textures/equine.rs",
            include_str!("../model_layers/textures/equine.rs"),
        ),
        ("textured/layers.rs", include_str!("../textured/layers.rs")),
        ("catalog.rs", include_str!("../catalog.rs")),
        ("instances.rs", include_str!("../instances.rs")),
        ("renderer exports.rs", include_str!("../../exports.rs")),
        (
            "render-types entity_textures.rs",
            include_str!("../../../../bbb-render-types/src/entity_textures.rs"),
        ),
        (
            "item-model item_runtime.rs",
            include_str!("../../../../bbb-item-model/src/item_runtime.rs"),
        ),
        (
            "item-model item_runtime/tables.rs",
            include_str!("../../../../bbb-item-model/src/item_runtime/tables.rs"),
        ),
        (
            "pack item_registry.rs",
            include_str!("../../../../bbb-pack/src/item_registry.rs"),
        ),
        (
            "world item_profiles.rs",
            include_str!("../../../../bbb-world/src/item_profiles.rs"),
        ),
        (
            "world entities.rs",
            include_str!("../../../../bbb-world/src/entities.rs"),
        ),
        (
            "world entities/state.rs",
            include_str!("../../../../bbb-world/src/entities/state.rs"),
        ),
        (
            "world entities/store.rs",
            include_str!("../../../../bbb-world/src/entities/store.rs"),
        ),
        (
            "world client/inventory.rs",
            include_str!("../../../../bbb-world/src/client/inventory.rs"),
        ),
        (
            "native resource_reload.rs",
            include_str!("../../../../bbb-native/src/resource_reload.rs"),
        ),
        (
            "native entity_scene.rs",
            include_str!("../../../../bbb-native/src/entity_scene.rs"),
        ),
        (
            "native entity_scene/instance.rs",
            include_str!("../../../../bbb-native/src/entity_scene/instance.rs"),
        ),
        (
            "native entity_scene/variants.rs",
            include_str!("../../../../bbb-native/src/entity_scene/variants.rs"),
        ),
    ];
    let forbidden = [
        "legacy_static_fallback",
        "uses_legacy_static_fallback",
        "fn armor_slot_texture_for_layer",
        "fn armor_layer_tint(",
        "fn horse_body_armor_texture_layers",
        "fn wolf_body_armor_texture_layers",
        "fn llama_body_decor_texture_ref",
        "fn happy_ghast_harness_texture_ref",
        "fn nautilus_body_armor_texture_ref",
        "fn humanoid_armor_layer_pass",
        "fn wings_layer_pass",
        "ArmorMaterialKind",
        "EntityArmorMaterial",
        "EntityEquipmentLayerTexture",
        "LlamaBodyDecorColor",
        "HappyGhastHarnessColor",
        "default_item_armor_materials",
        "default_item_wings_equipment_assets",
        "humanoid_armor_assets",
        "mount_body_armor_assets",
        "pub fn humanoid_armor_asset(",
        "pub fn mount_body_armor_asset(",
        "default_llama_body_decor_colors",
        "default_happy_ghast_harness_colors",
        "default_nautilus_body_armor_materials",
        "default_horse_body_armor_materials",
        "default_wolf_body_armor_materials",
        "item_has_humanoid_armor_asset",
        "item_equipment_asset_has_wings_layer",
        "item_equipment_asset_has_humanoid_layer",
        "item_equipment_wings_layer",
        "item_wings_equipment_assets_by_protocol_id",
        "equipment_asset_names.is_empty()",
        "chest_equipment_has_wings",
        "chest_equipment_has_humanoid",
        "chest_wings_layer",
    ];
    for (path, source) in production_sources {
        for symbol in forbidden {
            assert!(
                !source.contains(symbol),
                "{path} restored deleted equipment compatibility resolver `{symbol}`"
            );
        }
    }
    let textured = include_str!("../textured.rs");
    assert!(
        textured.contains("let equipment_assets = ResolvedEntityEquipmentAssetCatalog::default();"),
        "the generic mesh helper must make missing equipment explicit instead of selecting a second resolver"
    );
}

#[test]
fn resolved_empty_catalog_does_not_enable_legacy_static_equipment() {
    let equipment_assets = EntityEquipmentAssetCatalog::new(GENERATION, Vec::new())
        .resolve(&[])
        .unwrap();
    let meshes = render(
        EntityModelInstance::pig(
            700,
            [0.0, 64.0, 0.0],
            0.0,
            PigModelVariant::Temperate,
            false,
        ),
        &[],
        &equipment_assets,
        None,
    );

    assert!(
        meshes
            .submissions
            .iter()
            .all(|submission| submission.texture != PIG_SADDLE_TEXTURE_REF),
        "an intentionally empty selected resource catalog must not fall back to static equipment"
    );
}

#[test]
fn simple_equipment_consumers_select_every_resource_layer_type() {
    let layer_types = [
        EntityEquipmentLayerType::WolfBody,
        EntityEquipmentLayerType::HorseBody,
        EntityEquipmentLayerType::LlamaBody,
        EntityEquipmentLayerType::PigSaddle,
        EntityEquipmentLayerType::StriderSaddle,
        EntityEquipmentLayerType::CamelSaddle,
        EntityEquipmentLayerType::CamelHuskSaddle,
        EntityEquipmentLayerType::HorseSaddle,
        EntityEquipmentLayerType::DonkeySaddle,
        EntityEquipmentLayerType::MuleSaddle,
        EntityEquipmentLayerType::ZombieHorseSaddle,
        EntityEquipmentLayerType::SkeletonHorseSaddle,
        EntityEquipmentLayerType::HappyGhastBody,
        EntityEquipmentLayerType::NautilusSaddle,
        EntityEquipmentLayerType::NautilusBody,
    ];
    let textures: Vec<_> = layer_types
        .iter()
        .map(|layer_type| {
            EntityModelOwnedTextureRef::new(
                format!("example:equipment/{layer_type:?}.png"),
                layer_type.texture_size(),
            )
        })
        .collect();
    let owned_entries: Vec<_> = textures.iter().cloned().map(owned_entry).collect();
    let layers = layer_types
        .iter()
        .copied()
        .zip(
            textures
                .iter()
                .cloned()
                .map(|texture| vec![owned_layer(texture)]),
        )
        .collect();
    let equipment_assets = single_asset_catalog(layers, &owned_entries);
    let asset_id = Some(EntityEquipmentAssetId::new(GENERATION, 0));
    let cases = [
        (
            "wolf body",
            EntityModelInstance::wolf(710, [0.0, 64.0, 0.0], 0.0, false)
                .with_wolf_body_equipment_asset(asset_id),
        ),
        (
            "horse body",
            EntityModelInstance::horse(711, [0.0, 64.0, 0.0], 0.0, false)
                .with_equine_body_equipment_asset(asset_id),
        ),
        (
            "llama body",
            EntityModelInstance::llama(
                712,
                [0.0, 64.0, 0.0],
                0.0,
                LlamaModelFamily::Llama,
                LlamaVariant::Creamy,
                false,
                false,
            )
            .with_llama_body_equipment_asset(asset_id),
        ),
        (
            "pig saddle",
            EntityModelInstance::pig(
                713,
                [0.0, 64.0, 0.0],
                0.0,
                PigModelVariant::Temperate,
                false,
            )
            .with_pig_saddle_equipment_asset(asset_id),
        ),
        (
            "strider saddle",
            EntityModelInstance::strider(714, [0.0, 64.0, 0.0], 0.0, false, false)
                .with_strider_saddle_equipment_asset(asset_id),
        ),
        (
            "camel saddle",
            EntityModelInstance::camel(715, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, false)
                .with_camel_saddle_equipment_asset(asset_id),
        ),
        (
            "camel husk saddle",
            EntityModelInstance::camel(
                716,
                [0.0, 64.0, 0.0],
                0.0,
                CamelModelFamily::CamelHusk,
                false,
            )
            .with_camel_saddle_equipment_asset(asset_id),
        ),
        (
            "horse saddle",
            EntityModelInstance::horse(717, [0.0, 64.0, 0.0], 0.0, false)
                .with_equine_saddle_equipment_asset(asset_id),
        ),
        (
            "donkey saddle",
            EntityModelInstance::donkey(
                718,
                [0.0, 64.0, 0.0],
                0.0,
                DonkeyModelFamily::Donkey,
                false,
                false,
            )
            .with_equine_saddle_equipment_asset(asset_id),
        ),
        (
            "mule saddle",
            EntityModelInstance::donkey(
                719,
                [0.0, 64.0, 0.0],
                0.0,
                DonkeyModelFamily::Mule,
                false,
                false,
            )
            .with_equine_saddle_equipment_asset(asset_id),
        ),
        (
            "zombie horse saddle",
            EntityModelInstance::undead_horse(
                720,
                [0.0, 64.0, 0.0],
                0.0,
                UndeadHorseModelFamily::Zombie,
                false,
            )
            .with_equine_saddle_equipment_asset(asset_id),
        ),
        (
            "skeleton horse saddle",
            EntityModelInstance::undead_horse(
                721,
                [0.0, 64.0, 0.0],
                0.0,
                UndeadHorseModelFamily::Skeleton,
                false,
            )
            .with_equine_saddle_equipment_asset(asset_id),
        ),
        (
            "happy ghast body",
            EntityModelInstance::happy_ghast(722, [0.0, 64.0, 0.0], 0.0)
                .with_happy_ghast_body_equipment_asset(asset_id),
        ),
        (
            "nautilus saddle",
            EntityModelInstance::nautilus(723, [0.0, 64.0, 0.0], 0.0, false)
                .with_nautilus_saddle_equipment_asset(asset_id),
        ),
        (
            "nautilus body",
            EntityModelInstance::nautilus(724, [0.0, 64.0, 0.0], 0.0, false)
                .with_nautilus_body_equipment_asset(asset_id),
        ),
    ];

    for (expected_owned_texture, (label, instance)) in cases.into_iter().enumerate() {
        let meshes = render(instance, &owned_entries, &equipment_assets, None);
        let actual: Vec<_> = meshes
            .submissions
            .iter()
            .filter_map(|submission| submission.owned_texture)
            .collect();
        assert_eq!(
            actual,
            [u32::try_from(expected_owned_texture).unwrap()],
            "{label} selected the wrong resource equipment layer type"
        );
        assert!(
            !meshes.armor_cutout.vertices.is_empty(),
            "{label} did not emit its owned equipment geometry"
        );
    }
}

#[test]
fn simple_equipment_layers_preserve_skip_dye_foil_trim_and_submission_order() {
    let skipped = EntityModelOwnedTextureRef::new("example:equipment/skipped.png", [64, 64]);
    let defaulted = EntityModelOwnedTextureRef::new("example:equipment/defaulted.png", [64, 64]);
    let plain = EntityModelOwnedTextureRef::new("example:equipment/plain.png", [64, 64]);
    let owned_entries = vec![
        owned_entry(skipped.clone()),
        owned_entry(defaulted.clone()),
        owned_entry(plain.clone()),
    ];
    let equipment_assets = single_asset_catalog(
        BTreeMap::from([(
            EntityEquipmentLayerType::PigSaddle,
            vec![
                EntityEquipmentAssetLayer {
                    texture: EntityEquipmentAssetLayerTexture::Owned(skipped),
                    dyeable: Some(EntityEquipmentDyeable {
                        color_when_undyed: None,
                    }),
                    use_player_texture: false,
                },
                EntityEquipmentAssetLayer {
                    texture: EntityEquipmentAssetLayerTexture::Owned(defaulted),
                    dyeable: Some(EntityEquipmentDyeable {
                        color_when_undyed: Some(0x11_22_33),
                    }),
                    use_player_texture: false,
                },
                owned_layer(plain),
            ],
        )]),
        &owned_entries,
    );
    let asset_id = Some(EntityEquipmentAssetId::new(GENERATION, 0));
    let pig = || {
        EntityModelInstance::pig(
            730,
            [0.0, 64.0, 0.0],
            0.0,
            PigModelVariant::Temperate,
            false,
        )
        .with_pig_saddle_equipment_asset(asset_id)
    };

    let undyed = render(pig(), &owned_entries, &equipment_assets, None);
    let undyed_layers: Vec<_> = undyed
        .submissions
        .iter()
        .filter(|submission| submission.owned_texture.is_some())
        .collect();
    assert_eq!(undyed_layers.len(), 2);
    assert_eq!(undyed_layers[0].owned_texture, Some(1));
    assert_eq!(
        undyed_layers[0].tint,
        [
            0x11 as f32 / 255.0,
            0x22 as f32 / 255.0,
            0x33 as f32 / 255.0,
            1.0,
        ]
    );
    assert_eq!(undyed_layers[1].owned_texture, Some(2));
    assert_eq!(undyed_layers[1].tint, [1.0; 4]);

    let trim_texture = EntityDynamicTexture { handle: 71_001 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicEntityTextureImage {
        handle: trim_texture.handle,
        size: [64, 64],
        standalone_sampling: None,
        rgba: vec![0x66; 64 * 64 * 4],
    }])
    .unwrap()
    .0;
    let dyed = render(
        pig()
            .with_pig_saddle_dye(Some(0x44_55_66))
            .with_pig_saddle_foil(true)
            .with_pig_saddle_trim(Some(EntityArmorTrim {
                texture: trim_texture,
                decal: true,
            })),
        &owned_entries,
        &equipment_assets,
        Some(&dynamic_atlas),
    );
    let equipment_submissions: Vec<_> = dyed
        .submissions
        .iter()
        .filter(|submission| {
            submission.owned_texture.is_some()
                || submission.render_type == EntityModelLayerRenderType::ArmorEntityGlint
                || submission.dynamic_texture == Some(trim_texture)
        })
        .collect();
    assert_eq!(equipment_submissions.len(), 5);
    assert_eq!(equipment_submissions[0].owned_texture, Some(0));
    assert_eq!(
        equipment_submissions[0].tint,
        [
            0x44 as f32 / 255.0,
            0x55 as f32 / 255.0,
            0x66 as f32 / 255.0,
            1.0,
        ]
    );
    assert_eq!(
        equipment_submissions[1].render_type,
        EntityModelLayerRenderType::ArmorEntityGlint
    );
    assert_eq!(equipment_submissions[1].tint, equipment_submissions[0].tint);
    assert_eq!(equipment_submissions[2].owned_texture, Some(1));
    assert_eq!(equipment_submissions[3].owned_texture, Some(2));
    assert_eq!(
        equipment_submissions[4].render_type,
        EntityModelLayerRenderType::ArmorTrimDecalCutoutNoCull
    );
    assert_eq!(equipment_submissions[4].dynamic_texture, Some(trim_texture));
    for (index, submission) in equipment_submissions.iter().enumerate() {
        assert_eq!(
            (submission.order, submission.submit_sequence),
            (
                i32::try_from(index).unwrap(),
                u32::try_from(index + 1).unwrap()
            )
        );
    }
}

#[test]
fn custom_wolf_body_layers_keep_damage_cracks_last() {
    let texture = EntityModelOwnedTextureRef::new("example:equipment/wolf.png", [64, 32]);
    let owned_entries = vec![owned_entry(texture.clone())];
    let equipment_assets = single_asset_catalog(
        BTreeMap::from([(
            EntityEquipmentLayerType::WolfBody,
            vec![owned_layer(texture)],
        )]),
        &owned_entries,
    );
    let meshes = render(
        EntityModelInstance::wolf(740, [0.0, 64.0, 0.0], 0.0, false)
            .with_wolf_body_equipment_asset(Some(EntityEquipmentAssetId::new(GENERATION, 0)))
            .with_wolf_body_armor_crackiness(Some(WolfArmorCrackiness::High)),
        &owned_entries,
        &equipment_assets,
        None,
    );
    let equipment = meshes
        .submissions
        .iter()
        .find(|submission| submission.owned_texture == Some(0))
        .unwrap();
    let crack = meshes
        .submissions
        .iter()
        .find(|submission| {
            submission.texture == WOLF_ARMOR_CRACKINESS_HIGH_TEXTURE_REF
                && submission.render_type == EntityModelLayerRenderType::ArmorTranslucent
        })
        .unwrap();
    assert_eq!(
        (equipment.order, equipment.submit_sequence),
        (1, 1),
        "WOLF_BODY starts at the vanilla equipment order"
    );
    assert_eq!(
        crack.submit_sequence, 2,
        "damage cracks submit after every custom equipment pass"
    );
}

#[test]
fn trader_llama_projected_assets_select_adult_and_baby_catalog_entries() {
    let layer = |texture| EntityEquipmentAssetLayer {
        texture: EntityEquipmentAssetLayerTexture::Static(texture),
        dyeable: None,
        use_player_texture: false,
    };
    let equipment_assets = EntityEquipmentAssetCatalog::new(
        GENERATION,
        vec![
            EntityEquipmentAsset {
                layers: BTreeMap::from([(
                    EntityEquipmentLayerType::LlamaBody,
                    vec![layer(LLAMA_BODY_TRADER_TEXTURE_REF)],
                )]),
            },
            EntityEquipmentAsset {
                layers: BTreeMap::from([(
                    EntityEquipmentLayerType::LlamaBody,
                    vec![layer(LLAMA_BODY_TRADER_BABY_TEXTURE_REF)],
                )]),
            },
        ],
    )
    .resolve(&[])
    .unwrap();
    let trader = |entity_id, baby, index| {
        EntityModelInstance::llama(
            entity_id,
            [0.0, 64.0, 0.0],
            0.0,
            LlamaModelFamily::TraderLlama,
            LlamaVariant::Creamy,
            baby,
            false,
        )
        .with_llama_body_equipment_asset(Some(EntityEquipmentAssetId::new(GENERATION, index)))
    };

    for (instance, expected_texture) in [
        (trader(750, false, 0), LLAMA_BODY_TRADER_TEXTURE_REF),
        (trader(751, true, 1), LLAMA_BODY_TRADER_BABY_TEXTURE_REF),
    ] {
        let meshes = render(instance, &[], &equipment_assets, None);
        let decor = meshes
            .submissions
            .iter()
            .find(|submission| submission.texture == expected_texture)
            .expect("projected trader llama equipment asset");
        assert_eq!(decor.owned_texture, None);
        assert_eq!((decor.order, decor.submit_sequence), (1, 1));
    }
}
