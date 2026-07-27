use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use bbb_pack::{
    EquipmentAssetCatalog, EquipmentLayerType, PackRoots, ResourceLocation, SpriteImage,
};
use bbb_renderer::{
    entity_model_texture_refs, EntityEquipmentAsset, EntityEquipmentAssetCatalog,
    EntityEquipmentAssetLayer, EntityEquipmentAssetLayerTexture, EntityEquipmentDyeable,
    EntityEquipmentLayerType, EntityModelOwnedTextureImage, EntityModelOwnedTextureRef,
    EntityModelTextureImage, EntityModelTextureRef, ENTITY_SHADOW_TEXTURE_PATH,
};

use crate::item_models::special_item_pack_texture_descriptors;

pub(crate) fn load_entity_model_textures(
    renderer: &mut bbb_renderer::Renderer,
    roots: Option<&PackRoots>,
    equipment_assets: Option<&EquipmentAssetCatalog>,
) {
    let Some(roots) = roots else {
        tracing::warn!(
            "continuing without vanilla entity feature textures because pack roots are unavailable"
        );
        return;
    };
    let empty_equipment_assets = EquipmentAssetCatalog::default();
    if let Err(err) = try_load_entity_model_textures(
        renderer,
        roots,
        equipment_assets.unwrap_or(&empty_equipment_assets),
    ) {
        tracing::warn!(?err, "continuing without vanilla entity feature textures");
    }
}

pub(crate) fn try_load_entity_model_textures(
    renderer: &mut bbb_renderer::Renderer,
    roots: &PackRoots,
    equipment_assets: &EquipmentAssetCatalog,
) -> Result<()> {
    let images = load_entity_model_texture_images(roots)?;
    let mut owned_images = load_special_item_texture_images(roots)?
        .into_iter()
        .map(|image| (image.texture.path.to_string(), image))
        .collect::<BTreeMap<_, _>>();
    let equipment_assets =
        load_equipment_asset_resources(roots, equipment_assets, &mut owned_images);
    let owned_images = owned_images.into_values().collect::<Vec<_>>();
    let shadow = load_entity_shadow_texture_image(roots)?;
    renderer.upload_entity_model_textures_with_owned_and_equipment(
        &images,
        &owned_images,
        &equipment_assets,
    )?;
    renderer.upload_entity_shadow_texture(shadow.width, shadow.height, &shadow.rgba)?;
    if let Some(glint) = images
        .iter()
        .find(|image| image.texture.path == "textures/misc/enchanted_glint_item.png")
    {
        renderer.upload_item_glint_texture(
            glint.texture.size[0],
            glint.texture.size[1],
            &glint.rgba,
        )?;
    }
    tracing::info!(
        textures = images.len(),
        owned_textures = owned_images.len(),
        equipment_assets = equipment_assets.len(),
        "loaded vanilla entity model and shadow textures"
    );
    Ok(())
}

fn load_equipment_asset_resources(
    roots: &PackRoots,
    source: &EquipmentAssetCatalog,
    owned_images: &mut BTreeMap<String, EntityModelOwnedTextureImage>,
) -> EntityEquipmentAssetCatalog {
    let mut assets = Vec::with_capacity(source.len());
    for (expected_index, (_index, _asset_id, source_asset)) in source.indexed_assets().enumerate() {
        debug_assert_eq!(expected_index, _index as usize);
        let mut layers = BTreeMap::new();
        for (source_type, source_layers) in &source_asset.layers {
            let layer_type = entity_equipment_layer_type(*source_type);
            let logical_size = layer_type.texture_size();
            let converted = source_layers
                .iter()
                .map(|source_layer| {
                    let texture = static_entity_texture_ref(&source_layer.texture_location)
                        .map(EntityEquipmentAssetLayerTexture::Static)
                        .unwrap_or_else(|| {
                            owned_images
                                .entry(source_layer.texture_location.clone())
                                .or_insert_with(|| {
                                    load_owned_equipment_texture(
                                        roots,
                                        &source_layer.texture_location,
                                        logical_size,
                                    )
                                });
                            EntityEquipmentAssetLayerTexture::Owned(
                                EntityModelOwnedTextureRef::new(
                                    source_layer.texture_location.clone(),
                                    logical_size,
                                ),
                            )
                        });
                    EntityEquipmentAssetLayer {
                        texture,
                        dyeable: source_layer.dyeable.map(|dyeable| EntityEquipmentDyeable {
                            color_when_undyed: dyeable.color_when_undyed,
                        }),
                        use_player_texture: source_layer.use_player_texture,
                    }
                })
                .collect();
            layers.insert(layer_type, converted);
        }
        assets.push(EntityEquipmentAsset { layers });
    }
    EntityEquipmentAssetCatalog::new(assets)
}

fn static_entity_texture_ref(path: &str) -> Option<EntityModelTextureRef> {
    let location = ResourceLocation::parse(path).ok()?;
    (location.namespace() == "minecraft")
        .then_some(location.path())
        .and_then(|path| {
            entity_model_texture_refs()
                .iter()
                .copied()
                .find(|texture| texture.path == path)
        })
}

fn load_owned_equipment_texture(
    roots: &PackRoots,
    path: &str,
    logical_size: [u32; 2],
) -> EntityModelOwnedTextureImage {
    let image = (|| -> Result<SpriteImage> {
        let location = ResourceLocation::parse(path)
            .with_context(|| format!("parse equipment layer texture {path}"))?;
        let resources = roots.resource_stack();
        let resource = resources
            .get_resource(&location)
            .with_context(|| format!("missing equipment layer texture {path}"))?;
        let image = SpriteImage::from_png_file_with_resource_stack(
            entity_texture_id(path),
            resource.path,
            &resources,
        )?;
        if image.width == 0 || image.height == 0 {
            bail!("equipment layer texture {path} has zero-sized dimensions");
        }
        Ok(image)
    })()
    .unwrap_or_else(|error| {
        tracing::warn!(
            texture = path,
            ?error,
            "using missing texture for equipment asset layer"
        );
        special_item_missing_texture()
    });
    EntityModelOwnedTextureImage::with_physical_size(
        EntityModelOwnedTextureRef::new(path.to_owned(), logical_size),
        [image.width, image.height],
        image.rgba,
    )
}

fn entity_equipment_layer_type(layer_type: EquipmentLayerType) -> EntityEquipmentLayerType {
    match layer_type {
        EquipmentLayerType::Humanoid => EntityEquipmentLayerType::Humanoid,
        EquipmentLayerType::HumanoidLeggings => EntityEquipmentLayerType::HumanoidLeggings,
        EquipmentLayerType::HumanoidBaby => EntityEquipmentLayerType::HumanoidBaby,
        EquipmentLayerType::Wings => EntityEquipmentLayerType::Wings,
        EquipmentLayerType::WolfBody => EntityEquipmentLayerType::WolfBody,
        EquipmentLayerType::HorseBody => EntityEquipmentLayerType::HorseBody,
        EquipmentLayerType::LlamaBody => EntityEquipmentLayerType::LlamaBody,
        EquipmentLayerType::PigSaddle => EntityEquipmentLayerType::PigSaddle,
        EquipmentLayerType::StriderSaddle => EntityEquipmentLayerType::StriderSaddle,
        EquipmentLayerType::CamelSaddle => EntityEquipmentLayerType::CamelSaddle,
        EquipmentLayerType::CamelHuskSaddle => EntityEquipmentLayerType::CamelHuskSaddle,
        EquipmentLayerType::HorseSaddle => EntityEquipmentLayerType::HorseSaddle,
        EquipmentLayerType::DonkeySaddle => EntityEquipmentLayerType::DonkeySaddle,
        EquipmentLayerType::MuleSaddle => EntityEquipmentLayerType::MuleSaddle,
        EquipmentLayerType::ZombieHorseSaddle => EntityEquipmentLayerType::ZombieHorseSaddle,
        EquipmentLayerType::SkeletonHorseSaddle => EntityEquipmentLayerType::SkeletonHorseSaddle,
        EquipmentLayerType::HappyGhastBody => EntityEquipmentLayerType::HappyGhastBody,
        EquipmentLayerType::NautilusSaddle => EntityEquipmentLayerType::NautilusSaddle,
        EquipmentLayerType::NautilusBody => EntityEquipmentLayerType::NautilusBody,
    }
}

fn load_special_item_texture_images(
    roots: &PackRoots,
) -> Result<Vec<EntityModelOwnedTextureImage>> {
    let item_models = roots
        .load_item_model_catalog()
        .context("load item models for special texture residency")?;
    let mut images = BTreeMap::new();
    for descriptor in special_item_pack_texture_descriptors(item_models.special_models()) {
        let location = ResourceLocation::parse(&descriptor.path)
            .with_context(|| format!("parse special item texture {}", descriptor.path))?;
        let resources = roots.resource_stack();
        let image = (|| -> Result<SpriteImage> {
            let resource = resources
                .get_resource(&location)
                .with_context(|| format!("missing special item texture {}", descriptor.path))?;
            let image = SpriteImage::from_png_file_with_resource_stack(
                entity_texture_id(&descriptor.path),
                resource.path,
                &resources,
            )?;
            if image.width == 0 || image.height == 0 {
                bail!(
                    "special item texture {} has zero-sized dimensions",
                    descriptor.path
                );
            }
            Ok(image)
        })()
        .unwrap_or_else(|error| {
            tracing::warn!(
                texture = descriptor.path,
                ?error,
                "using missing texture for special item model"
            );
            special_item_missing_texture()
        });
        images.insert(
            descriptor.path.clone(),
            EntityModelOwnedTextureImage::with_physical_size(
                EntityModelOwnedTextureRef::new(descriptor.path, descriptor.logical_size),
                [image.width, image.height],
                image.rgba,
            ),
        );
    }
    for atlas_name in ["banner_patterns", "shield_patterns"] {
        let Ok(patterns) = roots.load_atlas_texture_images(atlas_name) else {
            continue;
        };
        for image in patterns {
            let Some(path) = special_item_pattern_texture_path(&image.id) else {
                continue;
            };
            if entity_model_texture_refs()
                .iter()
                .any(|texture| texture.path == path)
            {
                continue;
            }
            images.entry(path.clone()).or_insert_with(|| {
                EntityModelOwnedTextureImage::with_physical_size(
                    EntityModelOwnedTextureRef::new(path, [64, 64]),
                    [image.width, image.height],
                    image.rgba,
                )
            });
        }
    }
    Ok(images.into_values().collect())
}

fn special_item_pattern_texture_path(sprite_id: &str) -> Option<String> {
    let sprite = ResourceLocation::parse(sprite_id).ok()?;
    if sprite.id() == "minecraft:missingno" {
        return None;
    }
    let path = format!("textures/{}.png", sprite.path());
    Some(if sprite.namespace() == "minecraft" {
        path
    } else {
        format!("{}:{path}", sprite.namespace())
    })
}

fn special_item_missing_texture() -> SpriteImage {
    const SIZE: u32 = 16;
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let magenta = (x < SIZE / 2) ^ (y < SIZE / 2);
            rgba.extend_from_slice(if magenta {
                &[255, 0, 255, 255]
            } else {
                &[0, 0, 0, 255]
            });
        }
    }
    SpriteImage::new("minecraft:missingno", SIZE, SIZE, rgba)
        .expect("fixed special-item missing texture is valid")
}

fn load_entity_shadow_texture_image(roots: &PackRoots) -> Result<SpriteImage> {
    let location = ResourceLocation::parse(ENTITY_SHADOW_TEXTURE_PATH)?;
    let resources = roots.resource_stack();
    let resource = resources.get_resource(&location).with_context(|| {
        format!("missing entity shadow texture minecraft:{ENTITY_SHADOW_TEXTURE_PATH}")
    })?;
    SpriteImage::from_png_file_with_resource_stack(
        entity_texture_id(ENTITY_SHADOW_TEXTURE_PATH),
        resource.path,
        &resources,
    )
}

fn load_entity_model_texture_images(roots: &PackRoots) -> Result<Vec<EntityModelTextureImage>> {
    entity_model_texture_refs()
        .iter()
        .copied()
        .map(|texture| load_entity_model_texture_image(roots, texture))
        .collect()
}

fn load_entity_model_texture_image(
    roots: &PackRoots,
    texture: EntityModelTextureRef,
) -> Result<EntityModelTextureImage> {
    let location = ResourceLocation::parse(texture.path)?;
    let resources = roots.resource_stack();
    let resource = resources
        .get_resource(&location)
        .with_context(|| format!("missing entity texture minecraft:{}", texture.path))?;
    let image = SpriteImage::from_png_file_with_resource_stack(
        entity_texture_id(texture.path),
        resource.path,
        &resources,
    )?;
    if image.width == 0 || image.height == 0 {
        bail!("entity texture {} has zero-sized dimensions", texture.path);
    }
    Ok(EntityModelTextureImage::with_physical_size(
        texture,
        [image.width, image.height],
        image.rgba,
    ))
}

fn entity_texture_id(path: &str) -> String {
    let Ok(location) = ResourceLocation::parse(path) else {
        return format!("minecraft:{}", path.strip_suffix(".png").unwrap_or(path));
    };
    format!(
        "{}:{}",
        location.namespace(),
        location
            .path()
            .strip_suffix(".png")
            .unwrap_or(location.path())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_vanilla_entity_textures_from_resource_stack() {
        let temp = unique_temp_dir("bbb-entity-textures");
        let sources = temp.join("sources").join("26.1");
        for texture in entity_model_texture_refs() {
            let texture_path = sources.join("assets").join("minecraft").join(texture.path);
            std::fs::create_dir_all(texture_path.parent().unwrap()).unwrap();
            write_png(&texture_path, texture.size[0], texture.size[1]);
        }
        let roots = PackRoots::from_paths(temp.clone(), sources, temp.join("unused-assets"), None);

        let images = load_entity_model_texture_images(&roots).unwrap();

        assert_eq!(images.len(), entity_model_texture_refs().len());
        for (image, texture) in images.iter().zip(entity_model_texture_refs()) {
            assert_eq!(&image.texture, texture);
            assert_eq!(
                image.rgba.len(),
                usize::try_from(image.size[0] * image.size[1] * 4).unwrap()
            );
        }

        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn loads_pack_defined_entity_texture_resolution() {
        let temp = unique_temp_dir("bbb-entity-textures-resolution");
        let sources = temp.join("sources").join("26.1");
        for texture in entity_model_texture_refs() {
            let texture_path = sources.join("assets").join("minecraft").join(texture.path);
            std::fs::create_dir_all(texture_path.parent().unwrap()).unwrap();
            let [width, height] = if texture.path == "textures/entity/sheep/sheep.png" {
                [32, 32]
            } else {
                texture.size
            };
            write_png(&texture_path, width, height);
        }
        let roots = PackRoots::from_paths(temp.clone(), sources, temp.join("unused-assets"), None);

        let images = load_entity_model_texture_images(&roots).unwrap();

        let sheep = images
            .iter()
            .find(|image| image.texture.path == "textures/entity/sheep/sheep.png")
            .unwrap();
        assert_eq!(sheep.texture.size, [64, 32]);
        assert_eq!(sheep.size, [32, 32]);
        assert_eq!(sheep.rgba.len(), 32 * 32 * 4);
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn loads_vanilla_entity_shadow_texture_from_resource_stack() {
        let temp = unique_temp_dir("bbb-entity-shadow-texture");
        let sources = temp.join("sources").join("26.1");
        let texture_path = sources
            .join("assets")
            .join("minecraft")
            .join(ENTITY_SHADOW_TEXTURE_PATH);
        std::fs::create_dir_all(texture_path.parent().unwrap()).unwrap();
        write_png(&texture_path, 16, 8);
        let roots = PackRoots::from_paths(temp.clone(), sources, temp.join("unused-assets"), None);

        let image = load_entity_shadow_texture_image(&roots).unwrap();

        assert_eq!([image.width, image.height], [16, 8]);
        assert_eq!(image.rgba.len(), 16 * 8 * 4);
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn missing_special_item_texture_uses_owned_missing_image() {
        let temp = unique_temp_dir("bbb-special-item-missing-texture");
        let sources = temp.join("sources").join("26.1");
        let item_model = sources
            .join("assets")
            .join("minecraft")
            .join("items")
            .join("missing_bed.json");
        std::fs::create_dir_all(item_model.parent().unwrap()).unwrap();
        std::fs::write(
            item_model,
            r#"{
                "model": {
                    "type": "minecraft:special",
                    "base": "minecraft:item/missing_bed",
                    "model": {
                        "type": "minecraft:bed",
                        "texture": "example:missing",
                        "part": "head"
                    }
                }
            }"#,
        )
        .unwrap();
        let roots = PackRoots::from_paths(temp.clone(), sources, temp.join("unused-assets"), None);

        let images = load_special_item_texture_images(&roots).unwrap();

        assert_eq!(images.len(), 1);
        assert_eq!(
            images[0].texture.path.as_ref(),
            "example:textures/entity/bed/missing.png"
        );
        assert_eq!(images[0].texture.size, [64, 64]);
        assert_eq!(images[0].size, [16, 16]);
        assert_eq!(images[0].rgba.len(), 16 * 16 * 4);
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn special_pattern_atlas_sprites_supply_owned_gui_fallbacks() {
        let temp = unique_temp_dir("bbb-special-item-pattern-texture");
        let sources = temp.join("sources").join("26.1");
        let atlas = sources
            .join("assets")
            .join("minecraft")
            .join("atlases")
            .join("shield_patterns.json");
        std::fs::create_dir_all(atlas.parent().unwrap()).unwrap();
        std::fs::write(
            atlas,
            r#"{
                "sources": [{
                    "type": "minecraft:single",
                    "resource": "example:entity/shield/custom"
                }]
            }"#,
        )
        .unwrap();
        let texture = sources
            .join("assets")
            .join("example")
            .join("textures")
            .join("entity")
            .join("shield")
            .join("custom.png");
        std::fs::create_dir_all(texture.parent().unwrap()).unwrap();
        write_png(&texture, 32, 64);
        let roots = PackRoots::from_paths(temp.clone(), sources, temp.join("unused-assets"), None);

        let images = load_special_item_texture_images(&roots).unwrap();

        assert_eq!(images.len(), 1);
        assert_eq!(
            images[0].texture.path.as_ref(),
            "example:textures/entity/shield/custom.png"
        );
        assert_eq!(images[0].texture.size, [64, 64]);
        assert_eq!(images[0].size, [32, 64]);
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn equipment_catalog_reuses_static_and_loads_owned_layer_textures() {
        let temp = unique_temp_dir("bbb-equipment-layer-textures");
        let sources = temp.join("sources").join("26.1");
        let equipment = sources
            .join("assets")
            .join("minecraft")
            .join("equipment")
            .join("mixed.json");
        std::fs::create_dir_all(equipment.parent().unwrap()).unwrap();
        std::fs::write(
            equipment,
            r#"{
                "layers": {
                    "humanoid": [
                        {
                            "texture": "example:custom",
                            "dyeable": { "color_when_undyed": 1193046 }
                        },
                        { "texture": "example:missing" }
                    ],
                    "wings": [
                        {
                            "texture": "minecraft:elytra",
                            "use_player_texture": true
                        }
                    ]
                }
            }"#,
        )
        .unwrap();
        let custom = sources
            .join("assets")
            .join("example")
            .join("textures")
            .join("entity")
            .join("equipment")
            .join("humanoid")
            .join("custom.png");
        std::fs::create_dir_all(custom.parent().unwrap()).unwrap();
        write_png(&custom, 32, 16);
        let roots = PackRoots::from_paths(temp.clone(), sources, temp.join("unused-assets"), None);
        let source = roots.load_equipment_asset_catalog().unwrap();
        let mut owned = BTreeMap::new();

        let catalog = load_equipment_asset_resources(&roots, &source, &mut owned);

        let asset = catalog
            .asset(bbb_renderer::EntityEquipmentAssetId(0))
            .unwrap();
        let humanoid = asset.layers(EntityEquipmentLayerType::Humanoid);
        assert_eq!(humanoid.len(), 2);
        assert_eq!(
            humanoid[0].texture,
            EntityEquipmentAssetLayerTexture::Owned(EntityModelOwnedTextureRef::new(
                "example:textures/entity/equipment/humanoid/custom.png",
                [64, 32],
            ))
        );
        assert_eq!(
            humanoid[0].dyeable,
            Some(EntityEquipmentDyeable {
                color_when_undyed: Some(1_193_046)
            })
        );
        assert_eq!(
            owned["example:textures/entity/equipment/humanoid/custom.png"].size,
            [32, 16]
        );
        assert_eq!(
            owned["example:textures/entity/equipment/humanoid/missing.png"].size,
            [16, 16]
        );
        let wings = &asset.layers(EntityEquipmentLayerType::Wings)[0];
        assert!(matches!(
            wings.texture,
            EntityEquipmentAssetLayerTexture::Static(texture)
                if texture.path == "textures/entity/equipment/wings/elytra.png"
        ));
        assert!(wings.use_player_texture);
        std::fs::remove_dir_all(temp).unwrap();
    }

    fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
    }

    fn write_png(path: &std::path::Path, width: u32, height: u32) {
        let image = image::RgbaImage::from_pixel(width, height, image::Rgba([7, 11, 13, 255]));
        image.save(path).unwrap();
    }
}
