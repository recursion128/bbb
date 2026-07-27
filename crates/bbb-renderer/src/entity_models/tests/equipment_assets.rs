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
        )
        .with_pig_saddle(true),
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
fn trader_llama_fallback_survives_a_nonempty_equipment_catalog() {
    let texture = EntityModelOwnedTextureRef::new("example:equipment/llama.png", [128, 64]);
    let owned_entries = vec![owned_entry(texture.clone())];
    let equipment_assets = single_asset_catalog(
        BTreeMap::from([(
            EntityEquipmentLayerType::LlamaBody,
            vec![owned_layer(texture)],
        )]),
        &owned_entries,
    );
    let trader = |entity_id, baby| {
        EntityModelInstance::llama(
            entity_id,
            [0.0, 64.0, 0.0],
            0.0,
            LlamaModelFamily::TraderLlama,
            LlamaVariant::Creamy,
            baby,
            false,
        )
    };

    for (instance, expected_texture) in [
        (trader(750, false), LLAMA_BODY_TRADER_TEXTURE_REF),
        (trader(751, true), LLAMA_BODY_TRADER_BABY_TEXTURE_REF),
    ] {
        let meshes = render(instance, &owned_entries, &equipment_assets, None);
        let decor = meshes
            .submissions
            .iter()
            .find(|submission| submission.texture == expected_texture)
            .expect("trader llama decor fallback");
        assert_eq!(decor.owned_texture, None);
        assert_eq!((decor.order, decor.submit_sequence), (1, 1));
    }
}
