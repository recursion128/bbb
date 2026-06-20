use anyhow::{bail, Context, Result};
use bbb_pack::{PackRoots, ResourceLocation, SpriteImage};
use bbb_renderer::{entity_model_texture_refs, EntityModelTextureImage, EntityModelTextureRef};

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
    fn loads_vanilla_entity_textures_from_resource_stack() {
        let temp = unique_temp_dir("bbb-entity-textures");
        let sources = temp.join("sources").join("26.1");
        for texture in entity_model_texture_refs() {
            let texture_path = sources.join("assets").join("minecraft").join(texture.path);
            std::fs::create_dir_all(texture_path.parent().unwrap()).unwrap();
            write_png(&texture_path, texture.size[0], texture.size[1]);
        }
        let roots = PackRoots {
            mc_code_root: temp.clone(),
            sources_dir: sources,
            assets_dir: temp.join("unused-assets"),
            generated_assets_dir: None,
            resource_pack_dirs: Vec::new(),
        };

        let images = load_entity_model_texture_images(&roots).unwrap();

        assert_eq!(images.len(), 13);
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
        assert_eq!(images[5].texture.path, "textures/entity/wolf/wolf.png");
        assert_eq!(images[6].texture.path, "textures/entity/wolf/wolf_tame.png");
        assert_eq!(
            images[7].texture.path,
            "textures/entity/wolf/wolf_angry.png"
        );
        assert_eq!(images[8].texture.path, "textures/entity/wolf/wolf_baby.png");
        assert_eq!(
            images[9].texture.path,
            "textures/entity/wolf/wolf_tame_baby.png"
        );
        assert_eq!(
            images[10].texture.path,
            "textures/entity/wolf/wolf_angry_baby.png"
        );
        assert_eq!(
            images[11].texture.path,
            "textures/entity/wolf/wolf_collar.png"
        );
        assert_eq!(
            images[12].texture.path,
            "textures/entity/wolf/wolf_collar_baby.png"
        );
        for image in images {
            assert_eq!(
                image.rgba.len(),
                usize::try_from(image.texture.size[0] * image.texture.size[1] * 4).unwrap()
            );
        }

        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn rejects_entity_texture_with_wrong_dimensions() {
        let temp = unique_temp_dir("bbb-entity-textures-bad");
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
