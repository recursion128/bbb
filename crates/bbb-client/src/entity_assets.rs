use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::OnceLock;

use anyhow::{anyhow, bail, Context, Result};
use bbb_client_assets::{
    EquipmentAssetCatalog, EquipmentLayerType, PackRoots, ResourceLocation, SpriteImage,
};
use bbb_client_render::{
    entity_model_texture_refs, DynamicEntityTextureImage, EntityEquipmentAsset,
    EntityEquipmentAssetCatalog, EntityEquipmentAssetLayer, EntityEquipmentAssetLayerTexture,
    EntityEquipmentDyeable, EntityEquipmentLayerType, EntityModelOwnedTextureImage,
    EntityModelOwnedTextureRef, EntityModelTextureImage, EntityModelTextureRef,
    StandaloneTextureSampling, ENTITY_SHADOW_TEXTURE_PATH,
};

use bbb_render_types::{
    cat_variants::VANILLA_CAT_VARIANT_TEXTURE_REFS,
    temperature_animal_variants::{
        VANILLA_CHICKEN_VARIANT_TEXTURE_REFS, VANILLA_COW_VARIANT_TEXTURE_REFS,
        VANILLA_FROG_VARIANT_TEXTURE_REFS, VANILLA_PIG_VARIANT_TEXTURE_REFS,
    },
    wolf_variants::VANILLA_WOLF_VARIANT_TEXTURE_REFS,
    zombie_nautilus_variants::VANILLA_ZOMBIE_NAUTILUS_VARIANT_TEXTURE_REFS,
};

use crate::item_models::special_item_pack_texture_descriptors;

pub(crate) const MISSING_OWNED_ENTITY_TEXTURE_PATH: &str = "minecraft:missingno";
const MAX_OWNED_ENTITY_TEXTURES: usize = 8_192;

pub(crate) type EntityPatternAtlases = BTreeMap<&'static str, Vec<SpriteImage>>;

pub(crate) struct StagedEntityModelTextures {
    images: Vec<EntityModelTextureImage>,
    reusable: HashSet<&'static str>,
    patterns: EntityPatternAtlases,
    owned_images: Vec<EntityModelOwnedTextureImage>,
    owned_texture_paths: HashSet<String>,
    equipment_assets: EntityEquipmentAssetCatalog,
    shadow: SpriteImage,
}

#[derive(Default)]
pub(crate) struct PreparedEntityTextures {
    pub(crate) patterns: EntityPatternAtlases,
    variants: HashMap<String, EntityModelTextureImage>,
    owned_texture_paths: HashSet<String>,
}

impl PreparedEntityTextures {
    pub(crate) fn remaining_variants(&self) -> usize {
        self.variants.len()
    }

    pub(crate) fn has_pack_owned_texture(&self, path: &str) -> bool {
        self.owned_texture_paths.contains(path)
    }

    fn new(
        patterns: EntityPatternAtlases,
        images: Vec<EntityModelTextureImage>,
        reusable: &HashSet<&str>,
        owned_texture_paths: HashSet<String>,
    ) -> Self {
        let variants = images
            .into_iter()
            .filter(|image| reusable.contains(image.texture.path))
            .map(|image| {
                let asset_id =
                    entity_texture_id(image.texture.path.strip_prefix("textures/").unwrap());
                (asset_id, image)
            })
            .collect();
        Self {
            patterns,
            variants,
            owned_texture_paths,
        }
    }

    pub(crate) fn load_variant(
        &mut self,
        roots: Option<&PackRoots>,
        asset_id: &str,
        handle: u64,
    ) -> Result<DynamicEntityTextureImage> {
        let (size, sampling, rgba) = if let Some(image) = self.variants.remove(asset_id) {
            (image.size, image.sampling, image.rgba)
        } else {
            let image = roots
                .ok_or_else(|| anyhow!("pack roots are unavailable"))?
                .load_resource_texture_image(asset_id)?;
            (
                image.size,
                StandaloneTextureSampling::new(
                    image.texture_metadata.blur,
                    image.texture_metadata.clamp,
                ),
                image.rgba,
            )
        };
        Ok(DynamicEntityTextureImage {
            handle,
            size,
            standalone_sampling: Some(sampling),
            rgba,
        })
    }
}

pub(crate) fn load_entity_model_textures(
    renderer: &mut bbb_client_render::Renderer,
    roots: Option<&PackRoots>,
    equipment_assets: Option<&EquipmentAssetCatalog>,
    equipment_asset_generation: u64,
) -> PreparedEntityTextures {
    let Some(roots) = roots else {
        tracing::warn!(
            "continuing without vanilla entity feature textures because pack roots are unavailable"
        );
        return PreparedEntityTextures::default();
    };
    let empty_equipment_assets = EquipmentAssetCatalog::default();
    try_load_entity_model_textures(
        renderer,
        roots,
        equipment_assets.unwrap_or(&empty_equipment_assets),
        equipment_asset_generation,
    )
    .unwrap_or_else(|err| {
        tracing::warn!(?err, "continuing without vanilla entity feature textures");
        PreparedEntityTextures::default()
    })
}

pub(crate) fn prepare_entity_model_textures(
    roots: &PackRoots,
    equipment_assets: &EquipmentAssetCatalog,
    equipment_asset_generation: u64,
) -> Result<StagedEntityModelTextures> {
    let (images, reusable) = load_entity_model_texture_images(roots)?;
    let (special_images, patterns) = load_special_item_texture_images(roots)?;
    let mut owned_images = special_images
        .into_iter()
        .map(|image| (image.texture.path.to_string(), image))
        .collect::<BTreeMap<_, _>>();
    let equipment_assets = load_equipment_asset_resources(
        roots,
        equipment_assets,
        equipment_asset_generation,
        &mut owned_images,
    )?;
    if owned_images.len() > MAX_OWNED_ENTITY_TEXTURES {
        bail!(
            "owned entity texture count {} exceeds limit {MAX_OWNED_ENTITY_TEXTURES}",
            owned_images.len()
        );
    }
    let owned_texture_paths = owned_images.keys().cloned().collect::<HashSet<_>>();
    let owned_images = owned_images.into_values().collect::<Vec<_>>();
    let shadow = load_entity_shadow_texture_image(roots)?;
    Ok(StagedEntityModelTextures {
        images,
        reusable,
        patterns,
        owned_images,
        owned_texture_paths,
        equipment_assets,
        shadow,
    })
}

pub(crate) fn upload_entity_model_textures(
    renderer: &mut bbb_client_render::Renderer,
    staged: StagedEntityModelTextures,
) -> Result<PreparedEntityTextures> {
    let StagedEntityModelTextures {
        images,
        reusable,
        patterns,
        owned_images,
        owned_texture_paths,
        equipment_assets,
        shadow,
    } = staged;
    renderer.upload_entity_model_textures_with_owned_and_equipment(
        &images,
        &owned_images,
        &equipment_assets,
    )?;
    renderer.upload_entity_shadow_texture_with_sampling(
        shadow.width,
        shadow.height,
        &shadow.rgba,
        StandaloneTextureSampling::new(shadow.texture_metadata.blur, shadow.texture_metadata.clamp),
    )?;
    if let Some(glint) = images
        .iter()
        .find(|image| image.texture.path == "textures/misc/enchanted_glint_item.png")
    {
        renderer.upload_item_glint_texture_with_sampling(
            glint.size[0],
            glint.size[1],
            &glint.rgba,
            glint.sampling,
        )?;
    }
    if let Some(glint) = images
        .iter()
        .find(|image| image.texture.path == "textures/misc/enchanted_glint_armor.png")
    {
        renderer.upload_armor_glint_texture_with_sampling(
            glint.size[0],
            glint.size[1],
            &glint.rgba,
            glint.sampling,
        )?;
    }
    tracing::info!(
        textures = images.len(),
        owned_textures = owned_images.len(),
        equipment_assets = equipment_assets.len(),
        "loaded vanilla entity model and shadow textures"
    );
    Ok(PreparedEntityTextures::new(
        patterns,
        images,
        &reusable,
        owned_texture_paths,
    ))
}

pub(crate) fn try_load_entity_model_textures(
    renderer: &mut bbb_client_render::Renderer,
    roots: &PackRoots,
    equipment_assets: &EquipmentAssetCatalog,
    equipment_asset_generation: u64,
) -> Result<PreparedEntityTextures> {
    let staged =
        prepare_entity_model_textures(roots, equipment_assets, equipment_asset_generation)?;
    upload_entity_model_textures(renderer, staged)
}

fn ensure_shared_missing_owned_texture(
    owned_images: &mut BTreeMap<String, EntityModelOwnedTextureImage>,
) {
    owned_images
        .entry(MISSING_OWNED_ENTITY_TEXTURE_PATH.to_owned())
        .or_insert_with(|| {
            let missing = special_item_missing_texture();
            EntityModelOwnedTextureImage::with_physical_size(
                EntityModelOwnedTextureRef::new(
                    MISSING_OWNED_ENTITY_TEXTURE_PATH.to_owned(),
                    [missing.width, missing.height],
                ),
                [missing.width, missing.height],
                missing.rgba,
            )
        });
}

fn load_equipment_asset_resources(
    roots: &PackRoots,
    source: &EquipmentAssetCatalog,
    generation: u64,
    owned_images: &mut BTreeMap<String, EntityModelOwnedTextureImage>,
) -> Result<EntityEquipmentAssetCatalog> {
    let mut assets = Vec::with_capacity(source.len());
    let mut missing_paths = HashSet::<String>::new();
    for (expected_index, (_index, _asset_id, source_asset)) in source.indexed_assets().enumerate() {
        debug_assert_eq!(expected_index, _index as usize);
        let mut layers = BTreeMap::new();
        for (source_type, source_layers) in &source_asset.layers {
            let layer_type = entity_equipment_layer_type(*source_type);
            let logical_size = layer_type.texture_size();
            let mut converted = Vec::with_capacity(source_layers.len());
            for source_layer in source_layers {
                let texture = if let Some(static_ref) =
                    static_entity_texture_ref(&source_layer.texture_location)
                {
                    EntityEquipmentAssetLayerTexture::Static(static_ref)
                } else if owned_images.contains_key(&source_layer.texture_location) {
                    EntityEquipmentAssetLayerTexture::Owned(EntityModelOwnedTextureRef::new(
                        source_layer.texture_location.clone(),
                        logical_size,
                    ))
                } else if missing_paths.contains(&source_layer.texture_location) {
                    ensure_shared_missing_owned_texture(owned_images);
                    EntityEquipmentAssetLayerTexture::Owned(EntityModelOwnedTextureRef::new(
                        MISSING_OWNED_ENTITY_TEXTURE_PATH.to_owned(),
                        logical_size,
                    ))
                } else {
                    match try_load_owned_equipment_texture(
                        roots,
                        &source_layer.texture_location,
                        logical_size,
                    ) {
                        Ok(image) => {
                            if owned_images.len() >= MAX_OWNED_ENTITY_TEXTURES {
                                bail!(
                                    "owned entity texture count exceeds limit {MAX_OWNED_ENTITY_TEXTURES}"
                                );
                            }
                            owned_images.insert(source_layer.texture_location.clone(), image);
                            EntityEquipmentAssetLayerTexture::Owned(
                                EntityModelOwnedTextureRef::new(
                                    source_layer.texture_location.clone(),
                                    logical_size,
                                ),
                            )
                        }
                        Err(error) => {
                            tracing::warn!(
                                texture = source_layer.texture_location,
                                ?error,
                                "using shared missing texture for equipment asset layer"
                            );
                            missing_paths.insert(source_layer.texture_location.clone());
                            ensure_shared_missing_owned_texture(owned_images);
                            EntityEquipmentAssetLayerTexture::Owned(
                                EntityModelOwnedTextureRef::new(
                                    MISSING_OWNED_ENTITY_TEXTURE_PATH.to_owned(),
                                    logical_size,
                                ),
                            )
                        }
                    }
                };
                converted.push(EntityEquipmentAssetLayer {
                    texture,
                    dyeable: source_layer.dyeable.map(|dyeable| EntityEquipmentDyeable {
                        color_when_undyed: dyeable.color_when_undyed,
                    }),
                    use_player_texture: source_layer.use_player_texture,
                });
            }
            layers.insert(layer_type, converted);
        }
        assets.push(EntityEquipmentAsset { layers });
    }
    Ok(EntityEquipmentAssetCatalog::new(generation, assets))
}

fn static_entity_texture_index() -> &'static HashMap<&'static str, EntityModelTextureRef> {
    static INDEX: OnceLock<HashMap<&'static str, EntityModelTextureRef>> = OnceLock::new();
    INDEX.get_or_init(|| {
        entity_model_texture_refs()
            .iter()
            .copied()
            .map(|texture| (texture.path, texture))
            .collect()
    })
}

fn static_entity_texture_ref(path: &str) -> Option<EntityModelTextureRef> {
    let location = ResourceLocation::parse(path).ok()?;
    (location.namespace() == "minecraft")
        .then_some(location.path())
        .and_then(|path| static_entity_texture_index().get(path).copied())
}

fn try_load_owned_equipment_texture(
    roots: &PackRoots,
    path: &str,
    logical_size: [u32; 2],
) -> Result<EntityModelOwnedTextureImage> {
    let location = ResourceLocation::parse(path)
        .with_context(|| format!("parse equipment layer texture {path}"))?;
    let resources = roots.resource_stack();
    let resource = resources
        .get_resource(&location)
        .with_context(|| format!("missing equipment layer texture {path}"))?;
    let image = SpriteImage::from_pack_resource(entity_texture_id(path), &resource, &resources)?;
    if image.width == 0 || image.height == 0 {
        bail!("equipment layer texture {path} has zero-sized dimensions");
    }
    Ok(EntityModelOwnedTextureImage::with_physical_size(
        EntityModelOwnedTextureRef::new(path.to_owned(), logical_size),
        [image.width, image.height],
        image.rgba,
    ))
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
) -> Result<(Vec<EntityModelOwnedTextureImage>, EntityPatternAtlases)> {
    let item_models = roots
        .load_item_model_catalog()
        .context("load item models for special texture residency")?;
    let mut images = BTreeMap::new();
    let resources = roots.resource_stack();
    for descriptor in special_item_pack_texture_descriptors(item_models.special_models()) {
        let location = ResourceLocation::parse(&descriptor.path)
            .with_context(|| format!("parse special item texture {}", descriptor.path))?;
        let loaded = (|| -> Result<SpriteImage> {
            let resource = resources
                .get_resource(&location)
                .with_context(|| format!("missing special item texture {}", descriptor.path))?;
            let image = SpriteImage::from_pack_resource(
                entity_texture_id(&descriptor.path),
                &resource,
                &resources,
            )?;
            if image.width == 0 || image.height == 0 {
                bail!(
                    "special item texture {} has zero-sized dimensions",
                    descriptor.path
                );
            }
            Ok(image)
        })();
        match loaded {
            Ok(image) => {
                if images.len() >= MAX_OWNED_ENTITY_TEXTURES {
                    bail!("special item texture count exceeds limit {MAX_OWNED_ENTITY_TEXTURES}");
                }
                images.insert(
                    descriptor.path.clone(),
                    EntityModelOwnedTextureImage::with_physical_size(
                        EntityModelOwnedTextureRef::new(descriptor.path, descriptor.logical_size),
                        [image.width, image.height],
                        image.rgba,
                    ),
                );
            }
            Err(error) => {
                tracing::warn!(
                    texture = descriptor.path,
                    ?error,
                    "using shared missing texture for special item model"
                );
                ensure_shared_missing_owned_texture(&mut images);
            }
        }
    }
    let static_index = static_entity_texture_index();
    let mut pattern_atlases = EntityPatternAtlases::new();
    for atlas_name in ["banner_patterns", "shield_patterns"] {
        let Ok(patterns) = roots.load_atlas_texture_images(atlas_name) else {
            continue;
        };
        for image in &patterns {
            let Some(path) = special_item_pattern_texture_path(&image.id) else {
                continue;
            };
            if static_index.contains_key(path.as_str()) {
                continue;
            }
            if !images.contains_key(&path) && images.len() >= MAX_OWNED_ENTITY_TEXTURES {
                bail!("owned entity texture count exceeds limit {MAX_OWNED_ENTITY_TEXTURES}");
            }
            images.entry(path.clone()).or_insert_with(|| {
                EntityModelOwnedTextureImage::with_physical_size(
                    EntityModelOwnedTextureRef::new(path, [64, 64]),
                    [image.width, image.height],
                    image.rgba.clone(),
                )
            });
        }
        pattern_atlases.insert(atlas_name, patterns);
    }
    Ok((images.into_values().collect(), pattern_atlases))
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
    SpriteImage::from_pack_resource(
        entity_texture_id(ENTITY_SHADOW_TEXTURE_PATH),
        &resource,
        &resources,
    )
}

fn load_entity_model_texture_images(
    roots: &PackRoots,
) -> Result<(Vec<EntityModelTextureImage>, HashSet<&'static str>)> {
    let mut reusable: HashSet<_> = [
        VANILLA_CAT_VARIANT_TEXTURE_REFS.as_slice(),
        VANILLA_CHICKEN_VARIANT_TEXTURE_REFS.as_slice(),
        VANILLA_COW_VARIANT_TEXTURE_REFS.as_slice(),
        VANILLA_FROG_VARIANT_TEXTURE_REFS.as_slice(),
        VANILLA_PIG_VARIANT_TEXTURE_REFS.as_slice(),
        VANILLA_WOLF_VARIANT_TEXTURE_REFS.as_slice(),
        VANILLA_ZOMBIE_NAUTILUS_VARIANT_TEXTURE_REFS.as_slice(),
    ]
    .into_iter()
    .flatten()
    .map(|texture| texture.path)
    .collect();
    let images = entity_model_texture_refs()
        .iter()
        .copied()
        .map(|texture| load_entity_model_texture_image(roots, texture, &mut reusable))
        .collect::<Result<_>>()?;
    Ok((images, reusable))
}

fn load_entity_model_texture_image(
    roots: &PackRoots,
    texture: EntityModelTextureRef,
    reusable: &mut HashSet<&'static str>,
) -> Result<EntityModelTextureImage> {
    let location = ResourceLocation::parse(texture.path)?;
    let resources = roots.resource_stack();
    let resource = resources
        .get_resource(&location)
        .with_context(|| format!("missing entity texture minecraft:{}", texture.path))?;
    // Sprites use stack-selected metadata; standalone textures read the physical PNG's sidecar.
    if reusable.contains(texture.path) && resource.has_metadata() {
        reusable.remove(texture.path);
    }
    let image =
        SpriteImage::from_pack_resource(entity_texture_id(texture.path), &resource, &resources)?;
    if image.width == 0 || image.height == 0 {
        bail!("entity texture {} has zero-sized dimensions", texture.path);
    }
    Ok(EntityModelTextureImage::with_physical_size_and_sampling(
        texture,
        [image.width, image.height],
        image.rgba,
        StandaloneTextureSampling::new(image.texture_metadata.blur, image.texture_metadata.clamp),
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
    fn prepared_animal_texture_reuses_pixels_but_animated_variant_keeps_the_whole_sheet() {
        let temp = unique_temp_dir("bbb-prepared-animal-textures");
        let sources = temp.join("pack");
        let plain = VANILLA_COW_VARIANT_TEXTURE_REFS[0];
        let animated = VANILLA_COW_VARIANT_TEXTURE_REFS[1];
        let mut reusable = HashSet::from([plain.path, animated.path]);
        for (texture, height) in [(plain, 2), (animated, 4)] {
            let path = sources.join("assets/minecraft").join(texture.path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            write_png(&path, 2, height);
            if texture == animated {
                std::fs::write(
                    path.with_added_extension("mcmeta"),
                    br#"{"animation":{"width":2,"height":2},"texture":{"blur":true,"clamp":true}}"#,
                )
                .unwrap();
            }
        }
        let roots = fixture_pack_roots(&temp, sources);
        let images = [plain, animated].map(|texture| {
            load_entity_model_texture_image(&roots, texture, &mut reusable).unwrap()
        });
        assert_eq!(images[1].size, [2, 2]);
        let mut prepared = PreparedEntityTextures::new(
            Default::default(),
            images.into(),
            &reusable,
            HashSet::new(),
        );
        let charged = roots.resource_image_memory_observation().charged_rgba_bytes;
        let asset = |texture: EntityModelTextureRef| {
            entity_texture_id(texture.path.strip_prefix("textures/").unwrap())
        };
        let image = prepared
            .load_variant(Some(&roots), &asset(plain), 1)
            .unwrap();
        assert_eq!(image.size, [2, 2]);
        assert_eq!(image.rgba, [7, 11, 13, 255].repeat(4));
        assert_eq!(
            roots.resource_image_memory_observation().charged_rgba_bytes,
            charged
        );
        let image = prepared
            .load_variant(Some(&roots), &asset(animated), 2)
            .unwrap();
        assert_eq!(image.size, [2, 4]);
        assert_eq!(image.rgba, [7, 11, 13, 255].repeat(8));
        assert_eq!(
            image.standalone_sampling,
            Some(StandaloneTextureSampling::new(true, true))
        );
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn loads_vanilla_entity_textures_from_resource_stack() {
        let temp = unique_temp_dir("bbb-entity-textures");
        let sources = temp.join("sources").join("26.1");
        for texture in entity_model_texture_refs() {
            let texture_path = sources.join("assets").join("minecraft").join(texture.path);
            std::fs::create_dir_all(texture_path.parent().unwrap()).unwrap();
            write_png(&texture_path, texture.size[0], texture.size[1]);
            if texture.path == "textures/misc/enchanted_glint_item.png" {
                std::fs::write(
                    texture_path.with_file_name("enchanted_glint_item.png.mcmeta"),
                    br#"{"texture":{"blur":true}}"#,
                )
                .unwrap();
            } else if texture.path == "textures/misc/enchanted_glint_armor.png" {
                std::fs::write(
                    texture_path.with_file_name("enchanted_glint_armor.png.mcmeta"),
                    br#"{"texture":{"clamp":true}}"#,
                )
                .unwrap();
            }
        }
        let roots = fixture_pack_roots(&temp, sources);

        let (images, _) = load_entity_model_texture_images(&roots).unwrap();

        assert_eq!(images.len(), entity_model_texture_refs().len());
        for (image, texture) in images.iter().zip(entity_model_texture_refs()) {
            assert_eq!(&image.texture, texture);
            assert_eq!(
                image.rgba.len(),
                usize::try_from(image.size[0] * image.size[1] * 4).unwrap()
            );
        }
        let item_glint = images
            .iter()
            .find(|image| image.texture.path == "textures/misc/enchanted_glint_item.png")
            .unwrap();
        assert_eq!(
            item_glint.sampling,
            StandaloneTextureSampling::new(true, false)
        );
        let armor_glint = images
            .iter()
            .find(|image| image.texture.path == "textures/misc/enchanted_glint_armor.png")
            .unwrap();
        assert_eq!(
            armor_glint.sampling,
            StandaloneTextureSampling::new(false, true)
        );

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
        let roots = fixture_pack_roots(&temp, sources);

        let (images, _) = load_entity_model_texture_images(&roots).unwrap();

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
        std::fs::write(
            texture_path.with_file_name("shadow.png.mcmeta"),
            br#"{"texture":{"blur":true,"clamp":true}}"#,
        )
        .unwrap();
        let roots = fixture_pack_roots(&temp, sources);

        let image = load_entity_shadow_texture_image(&roots).unwrap();

        assert_eq!([image.width, image.height], [16, 8]);
        assert!(image.texture_metadata.blur);
        assert!(image.texture_metadata.clamp);
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
        let roots = fixture_pack_roots(&temp, sources);

        let (images, _) = load_special_item_texture_images(&roots).unwrap();

        assert_eq!(images.len(), 1);
        assert_eq!(
            images[0].texture.path.as_ref(),
            MISSING_OWNED_ENTITY_TEXTURE_PATH
        );
        assert_eq!(images[0].texture.size, [16, 16]);
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
        let roots = fixture_pack_roots(&temp, sources);

        let (images, patterns) = load_special_item_texture_images(&roots).unwrap();
        let shield = &patterns["shield_patterns"];
        assert!(shield.iter().any(|image| image.id == "minecraft:missingno"));
        let sprite = shield
            .iter()
            .find(|image| image.id == "example:entity/shield/custom")
            .unwrap();
        assert_eq!(sprite.rgba, images[0].rgba);

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
        let roots = fixture_pack_roots(&temp, sources);
        let source = roots.load_equipment_asset_catalog().unwrap();
        let mut owned = BTreeMap::new();

        let catalog = load_equipment_asset_resources(&roots, &source, 9, &mut owned).unwrap();

        let asset = catalog
            .asset(bbb_client_render::EntityEquipmentAssetId::new(9, 0))
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
            humanoid[1].texture,
            EntityEquipmentAssetLayerTexture::Owned(EntityModelOwnedTextureRef::new(
                MISSING_OWNED_ENTITY_TEXTURE_PATH,
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
        assert_eq!(owned[MISSING_OWNED_ENTITY_TEXTURE_PATH].size, [16, 16]);
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
        let nanos = bbb_client_clock::SystemTime::now()
            .duration_since(bbb_client_clock::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
    }

    fn fixture_pack_roots(temp: &std::path::Path, pack: std::path::PathBuf) -> PackRoots {
        let vanilla = temp.join("empty-vanilla");
        PackRoots::from_paths(
            temp.to_path_buf(),
            &vanilla,
            vanilla.join("assets/minecraft"),
        )
        .with_resource_pack_dirs([pack])
    }

    fn write_png(path: &std::path::Path, width: u32, height: u32) {
        let image = image::RgbaImage::from_pixel(width, height, image::Rgba([7, 11, 13, 255]));
        image.save(path).unwrap();
    }
}
