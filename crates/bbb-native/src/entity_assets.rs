use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use bbb_pack::{PackRoots, ResourceLocation, SpriteImage};
use bbb_renderer::{
    entity_model_texture_refs, EntityModelOwnedTextureImage, EntityModelOwnedTextureRef,
    EntityModelTextureImage, EntityModelTextureRef, ENTITY_SHADOW_TEXTURE_PATH,
};

use crate::item_models::special_item_pack_texture_descriptors;

pub(crate) fn load_entity_model_textures(
    renderer: &mut bbb_renderer::Renderer,
    roots: Option<&PackRoots>,
) {
    let Some(roots) = roots else {
        tracing::warn!(
            "continuing without vanilla entity feature textures because pack roots are unavailable"
        );
        return;
    };
    if let Err(err) = try_load_entity_model_textures(renderer, roots) {
        tracing::warn!(?err, "continuing without vanilla entity feature textures");
    }
}

pub(crate) fn try_load_entity_model_textures(
    renderer: &mut bbb_renderer::Renderer,
    roots: &PackRoots,
) -> Result<()> {
    let images = load_entity_model_texture_images(roots)?;
    let owned_images = load_special_item_texture_images(roots)?;
    let shadow = load_entity_shadow_texture_image(roots)?;
    renderer.upload_entity_model_textures_with_owned(&images, &owned_images)?;
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
        special_textures = owned_images.len(),
        "loaded vanilla entity model and shadow textures"
    );
    Ok(())
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
