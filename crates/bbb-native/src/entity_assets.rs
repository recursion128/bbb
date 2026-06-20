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

        assert_eq!(
            images
                .iter()
                .map(|image| image.texture.path)
                .collect::<Vec<_>>(),
            vec![
                "textures/entity/player/wide/steve.png",
                "textures/entity/player/slim/steve.png",
                "textures/entity/sheep/sheep.png",
                "textures/entity/sheep/sheep_baby.png",
                "textures/entity/sheep/sheep_wool_undercoat.png",
                "textures/entity/sheep/sheep_wool.png",
                "textures/entity/sheep/sheep_wool_baby.png",
                "textures/entity/wolf/wolf.png",
                "textures/entity/wolf/wolf_tame.png",
                "textures/entity/wolf/wolf_angry.png",
                "textures/entity/wolf/wolf_baby.png",
                "textures/entity/wolf/wolf_tame_baby.png",
                "textures/entity/wolf/wolf_angry_baby.png",
                "textures/entity/wolf/wolf_collar.png",
                "textures/entity/wolf/wolf_collar_baby.png",
                "textures/entity/goat/goat.png",
                "textures/entity/goat/goat_baby.png",
                "textures/entity/bear/polarbear.png",
                "textures/entity/bear/polarbear_baby.png",
                "textures/entity/hoglin/hoglin.png",
                "textures/entity/hoglin/hoglin_baby.png",
                "textures/entity/hoglin/zoglin.png",
                "textures/entity/hoglin/zoglin_baby.png",
                "textures/entity/chicken/chicken_temperate.png",
                "textures/entity/chicken/chicken_temperate_baby.png",
                "textures/entity/chicken/chicken_warm.png",
                "textures/entity/chicken/chicken_warm_baby.png",
                "textures/entity/chicken/chicken_cold.png",
                "textures/entity/chicken/chicken_cold_baby.png",
                "textures/entity/pig/pig_temperate.png",
                "textures/entity/pig/pig_temperate_baby.png",
                "textures/entity/pig/pig_warm.png",
                "textures/entity/pig/pig_warm_baby.png",
                "textures/entity/pig/pig_cold.png",
                "textures/entity/pig/pig_cold_baby.png",
                "textures/entity/cow/cow_temperate.png",
                "textures/entity/cow/cow_temperate_baby.png",
                "textures/entity/cow/cow_warm.png",
                "textures/entity/cow/cow_warm_baby.png",
                "textures/entity/cow/cow_cold.png",
                "textures/entity/cow/cow_cold_baby.png",
                "textures/entity/skeleton/skeleton.png",
                "textures/entity/skeleton/stray.png",
                "textures/entity/skeleton/stray_overlay.png",
                "textures/entity/skeleton/parched.png",
                "textures/entity/skeleton/wither_skeleton.png",
                "textures/entity/skeleton/bogged.png",
                "textures/entity/skeleton/bogged_overlay.png",
                "textures/entity/creeper/creeper.png",
                "textures/entity/spider/spider.png",
                "textures/entity/spider/cave_spider.png",
                "textures/entity/spider/spider_eyes.png",
                "textures/entity/enderman/enderman.png",
                "textures/entity/enderman/enderman_eyes.png",
                "textures/entity/slime/slime.png",
                "textures/entity/slime/magmacube.png",
                "textures/entity/boat/acacia.png",
                "textures/entity/chest_boat/acacia.png",
                "textures/entity/boat/bamboo.png",
                "textures/entity/chest_boat/bamboo.png",
                "textures/entity/boat/birch.png",
                "textures/entity/chest_boat/birch.png",
                "textures/entity/boat/cherry.png",
                "textures/entity/chest_boat/cherry.png",
                "textures/entity/boat/dark_oak.png",
                "textures/entity/chest_boat/dark_oak.png",
                "textures/entity/boat/jungle.png",
                "textures/entity/chest_boat/jungle.png",
                "textures/entity/boat/mangrove.png",
                "textures/entity/chest_boat/mangrove.png",
                "textures/entity/boat/oak.png",
                "textures/entity/chest_boat/oak.png",
                "textures/entity/boat/pale_oak.png",
                "textures/entity/chest_boat/pale_oak.png",
                "textures/entity/boat/spruce.png",
                "textures/entity/chest_boat/spruce.png",
            ]
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
