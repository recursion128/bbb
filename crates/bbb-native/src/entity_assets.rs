use anyhow::{bail, Context, Result};
use bbb_pack::{PackRoots, ResourceLocation, SpriteImage};
use bbb_renderer::{sheep_entity_texture_refs, EntityModelTextureImage, EntityModelTextureRef};

pub(crate) fn load_entity_model_textures(
    renderer: &mut bbb_renderer::Renderer,
    roots: Option<&PackRoots>,
) {
    let Some(roots) = roots else {
        tracing::warn!(
            "continuing without vanilla entity textures because pack roots are unavailable"
        );
        return;
    };
    if let Err(err) = try_load_entity_model_textures(renderer, roots) {
        tracing::warn!(?err, "continuing without vanilla entity textures");
    }
}

fn try_load_entity_model_textures(
    renderer: &mut bbb_renderer::Renderer,
    roots: &PackRoots,
) -> Result<()> {
    let images = load_entity_model_texture_images(roots)?;
    renderer.upload_entity_model_textures(&images)?;
    tracing::info!(
        textures = images.len(),
        "loaded vanilla entity model textures"
    );
    Ok(())
}

fn load_entity_model_texture_images(roots: &PackRoots) -> Result<Vec<EntityModelTextureImage>> {
    sheep_entity_texture_refs()
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
    let resource = roots
        .resource_stack()
        .get_resource(&location)
        .with_context(|| format!("missing entity texture minecraft:{}", texture.path))?;
    let image = SpriteImage::from_png_file(entity_texture_id(texture.path), resource.path)?;
    if [image.width, image.height] != texture.size {
        bail!(
            "entity texture {} has {}x{}, expected {}x{}",
            texture.path,
            image.width,
            image.height,
            texture.size[0],
            texture.size[1]
        );
    }
    Ok(EntityModelTextureImage::new(texture, image.rgba))
}

fn entity_texture_id(path: &str) -> String {
    let path = path.strip_suffix(".png").unwrap_or(path);
    format!("minecraft:{path}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_vanilla_sheep_entity_textures_from_resource_stack() {
        let temp = unique_temp_dir("bbb-entity-textures");
        let sources = temp.join("sources").join("26.1");
        let texture_dir = sources
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("entity")
            .join("sheep");
        std::fs::create_dir_all(&texture_dir).unwrap();
        for texture in sheep_entity_texture_refs() {
            write_png(
                &texture_dir.join(texture.path.rsplit('/').next().unwrap()),
                texture.size[0],
                texture.size[1],
            );
        }
        let roots = PackRoots {
            mc_code_root: temp.clone(),
            sources_dir: sources,
            assets_dir: temp.join("unused-assets"),
            generated_assets_dir: None,
            resource_pack_dirs: Vec::new(),
        };

        let images = load_entity_model_texture_images(&roots).unwrap();

        assert_eq!(images.len(), 5);
        assert_eq!(images[0].texture.path, "textures/entity/sheep/sheep.png");
        assert_eq!(
            images[1].texture.path,
            "textures/entity/sheep/sheep_baby.png"
        );
        assert_eq!(
            images[2].texture.path,
            "textures/entity/sheep/sheep_wool_undercoat.png"
        );
        assert_eq!(
            images[3].texture.path,
            "textures/entity/sheep/sheep_wool.png"
        );
        assert_eq!(
            images[4].texture.path,
            "textures/entity/sheep/sheep_wool_baby.png"
        );
        for image in images {
            assert_eq!(image.texture.size, [64, 32]);
            assert_eq!(image.rgba.len(), 64 * 32 * 4);
        }

        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn rejects_sheep_entity_texture_with_wrong_dimensions() {
        let temp = unique_temp_dir("bbb-entity-textures-bad");
        let sources = temp.join("sources").join("26.1");
        let texture_dir = sources
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("entity")
            .join("sheep");
        std::fs::create_dir_all(&texture_dir).unwrap();
        for texture in sheep_entity_texture_refs() {
            let [width, height] = if texture.path == "textures/entity/sheep/sheep.png" {
                [32, 32]
            } else {
                texture.size
            };
            write_png(
                &texture_dir.join(texture.path.rsplit('/').next().unwrap()),
                width,
                height,
            );
        }
        let roots = PackRoots {
            mc_code_root: temp.clone(),
            sources_dir: sources,
            assets_dir: temp.join("unused-assets"),
            generated_assets_dir: None,
            resource_pack_dirs: Vec::new(),
        };

        let err = load_entity_model_texture_images(&roots).unwrap_err();

        assert!(err.to_string().contains("expected 64x32"));
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
