use anyhow::{bail, Context, Result};
use bbb_pack::{PackRoots, ResourceLocation, SpriteImage};
use bbb_renderer::{
    entity_model_texture_refs, EntityModelTextureImage, EntityModelTextureRef,
    ENTITY_SHADOW_TEXTURE_PATH,
};

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
    let shadow = load_entity_shadow_texture_image(roots)?;
    renderer.upload_entity_model_textures(&images)?;
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
        "loaded vanilla entity model and shadow textures"
    );
    Ok(())
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
    let path = path.strip_suffix(".png").unwrap_or(path);
    format!("minecraft:{path}")
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
