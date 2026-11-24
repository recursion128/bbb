use super::*;
use crate::entity_models::equipment::ResolvedEntityEquipmentAssetCatalog;
use crate::DynamicEntityTextureImage;
use std::collections::BTreeMap;

const GENERATION: u64 = 71;

fn static_atlas() -> EntityModelTextureAtlasLayout {
    let images: Vec<_> = [
        PIG_TEMPERATE_TEXTURE_REF,
        PIG_SADDLE_TEXTURE_REF,
        ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
        WOLF_ARMOR_CRACKINESS_HIGH_TEXTURE_REF,
        LLAMA_BODY_TRADER_TEXTURE_REF,
        LLAMA_BODY_TRADER_BABY_TEXTURE_REF,
    ]
    .into_iter()
    .map(|texture| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(texture, vec![0; len])
    })
    .collect();
    build_entity_model_texture_atlas(&images).unwrap().0
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
        &static_atlas(),
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
        ),
        &[],
        &equipment_assets,
        None,
    );

    assert!(meshes.armor_cutout.vertices.is_empty());
    assert!(!meshes.cutout.vertices.is_empty());
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
    let owned_entries: Vec<_> = textures
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, texture)| {
            let mut entry = owned_entry(texture);
            entry.uv = EntityModelUvRect {
                min: [index as f32 / layer_types.len() as f32, 0.0],
                max: [(index + 1) as f32 / layer_types.len() as f32, 1.0],
            };
            entry
        })
        .collect();
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
                .with(|s| s.kind.wolf_mut().body_equipment_asset = asset_id),
        ),
        (
            "horse body",
            EntityModelInstance::horse(711, [0.0, 64.0, 0.0], 0.0, false)
                .with(|s| s.kind.equine_mut().body_equipment_asset = asset_id),
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
            .with(|s| s.kind.llama_mut().body_equipment_asset = asset_id),
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
            .with(|s| s.kind.pig_mut().saddle_equipment_asset = asset_id),
        ),
        (
            "strider saddle",
            EntityModelInstance::strider(714, [0.0, 64.0, 0.0], 0.0, false, false)
                .with(|s| s.kind.strider_mut().saddle_equipment_asset = asset_id),
        ),
        (
            "camel saddle",
            EntityModelInstance::camel(715, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, false)
                .with(|s| s.kind.camel_mut().saddle_equipment_asset = asset_id),
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
            .with(|s| s.kind.camel_mut().saddle_equipment_asset = asset_id),
        ),
        (
            "horse saddle",
            EntityModelInstance::horse(717, [0.0, 64.0, 0.0], 0.0, false)
                .with(|s| s.kind.equine_mut().saddle_equipment_asset = asset_id),
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
            .with(|s| s.kind.equine_mut().saddle_equipment_asset = asset_id),
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
            .with(|s| s.kind.equine_mut().saddle_equipment_asset = asset_id),
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
            .with(|s| s.kind.equine_mut().saddle_equipment_asset = asset_id),
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
            .with(|s| s.kind.equine_mut().saddle_equipment_asset = asset_id),
        ),
        (
            "happy ghast body",
            EntityModelInstance::happy_ghast(722, [0.0, 64.0, 0.0], 0.0)
                .with(|s| s.kind.happy_ghast_mut().body_equipment_asset = asset_id),
        ),
        (
            "nautilus saddle",
            EntityModelInstance::nautilus(723, [0.0, 64.0, 0.0], 0.0, false)
                .with(|s| s.kind.nautilus_mut().saddle_equipment_asset = asset_id),
        ),
        (
            "nautilus body",
            EntityModelInstance::nautilus(724, [0.0, 64.0, 0.0], 0.0, false)
                .with(|s| s.kind.nautilus_mut().body_equipment_asset = asset_id),
        ),
    ];

    for (expected_owned_texture, (label, instance)) in cases.into_iter().enumerate() {
        let meshes = render(instance, &owned_entries, &equipment_assets, None);
        assert!(
            !meshes.armor_cutout.vertices.is_empty(),
            "{label} did not emit its owned equipment geometry"
        );
        let mut shifted_entries = owned_entries.clone();
        let uv = &mut shifted_entries[expected_owned_texture].uv;
        uv.min[0] += 0.125;
        uv.max[0] += 0.125;
        uv.min[1] += 0.25;
        uv.max[1] += 0.25;
        let shifted = render(instance, &shifted_entries, &equipment_assets, None);
        assert_eq!(
            meshes.armor_cutout.vertices.len(),
            shifted.armor_cutout.vertices.len()
        );
        for (original, shifted) in meshes
            .armor_cutout
            .vertices
            .iter()
            .zip(&shifted.armor_cutout.vertices)
        {
            assert!(
                (shifted.uv[0] - original.uv[0] - 0.125).abs() < 1.0e-5,
                "{label} selected the wrong resource equipment layer type"
            );
            assert!(
                (shifted.uv[1] - original.uv[1] - 0.25).abs() < 1.0e-5,
                "{label} selected the wrong resource equipment layer type"
            );
        }
    }
}

#[test]
fn simple_equipment_layers_preserve_skip_dye_foil_trim_and_layer_geometry() {
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
        .with(|s| s.kind.pig_mut().saddle_equipment_asset = asset_id)
    };

    let undyed = render(pig(), &owned_entries, &equipment_assets, None);
    let layer_vertices = undyed.armor_cutout.vertices.len() / 2;
    assert!(layer_vertices > 0);
    let default_tint = [
        0x11 as f32 / 255.0,
        0x22 as f32 / 255.0,
        0x33 as f32 / 255.0,
        1.0,
    ];
    assert!(undyed.armor_cutout.vertices[..layer_vertices]
        .iter()
        .all(|vertex| vertex.tint == default_tint));
    assert!(undyed.armor_cutout.vertices[layer_vertices..]
        .iter()
        .all(|vertex| vertex.tint == [1.0; 4]));

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
        pig().with(|s| {
            s.kind.pig_mut().saddle_dye = Some(0x44_55_66);
            s.kind.pig_mut().saddle_foil = true;
            s.kind.pig_mut().saddle_trim = Some(EntityArmorTrim {
                texture: trim_texture,
                decal: true,
            });
        }),
        &owned_entries,
        &equipment_assets,
        Some(&dynamic_atlas),
    );
    let dye_tint = [
        0x44 as f32 / 255.0,
        0x55 as f32 / 255.0,
        0x66 as f32 / 255.0,
        1.0,
    ];
    assert_eq!(dyed.armor_cutout.vertices.len(), layer_vertices * 3);
    assert!(dyed.armor_cutout.vertices[..layer_vertices * 2]
        .iter()
        .all(|vertex| vertex.tint == dye_tint));
    assert!(dyed.armor_cutout.vertices[layer_vertices * 2..]
        .iter()
        .all(|vertex| vertex.tint == [1.0; 4]));
    assert_eq!(dyed.armor_entity_glint.vertices.len(), layer_vertices);
    assert!(dyed
        .armor_entity_glint
        .vertices
        .iter()
        .all(|vertex| vertex.tint == dye_tint));
    assert_eq!(
        dyed.dynamic_player_texture_armor_trim_decal.vertices.len(),
        layer_vertices
    );
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
        EntityModelInstance::wolf(740, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
            s.kind.wolf_mut().body_equipment_asset =
                Some(EntityEquipmentAssetId::new(GENERATION, 0));
            s.kind.wolf_mut().body_armor_crackiness = WolfArmorCrackiness::High;
        }),
        &owned_entries,
        &equipment_assets,
        None,
    );
    assert!(!meshes.armor_cutout.vertices.is_empty());
    assert!(!meshes.armor_translucent.vertices.is_empty());
    let atlas = static_atlas();
    let uv = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == WOLF_ARMOR_CRACKINESS_HIGH_TEXTURE_REF)
        .unwrap()
        .uv;
    assert!(meshes.armor_translucent.vertices.iter().all(|vertex| {
        vertex.uv[0] >= uv.min[0]
            && vertex.uv[0] <= uv.max[0]
            && vertex.uv[1] >= uv.min[1]
            && vertex.uv[1] <= uv.max[1]
    }));
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
        .with(|s| {
            s.kind.llama_mut().body_equipment_asset =
                Some(EntityEquipmentAssetId::new(GENERATION, index))
        })
    };

    for (instance, expected_texture) in [
        (trader(750, false, 0), LLAMA_BODY_TRADER_TEXTURE_REF),
        (trader(751, true, 1), LLAMA_BODY_TRADER_BABY_TEXTURE_REF),
    ] {
        let meshes = render(instance, &[], &equipment_assets, None);
        assert!(!meshes.armor_cutout.vertices.is_empty());
        let atlas = static_atlas();
        let uv = atlas
            .entries
            .iter()
            .find(|entry| entry.texture == expected_texture)
            .unwrap()
            .uv;
        assert!(meshes.armor_cutout.vertices.iter().all(|vertex| {
            vertex.uv[0] >= uv.min[0]
                && vertex.uv[0] <= uv.max[0]
                && vertex.uv[1] >= uv.min[1]
                && vertex.uv[1] <= uv.max[1]
        }));
    }
}
