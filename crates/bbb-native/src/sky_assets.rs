use anyhow::{Context, Result};
use bbb_pack::{PackRoots, ResourceLocation, SpriteImage};

const END_SKY_TEXTURE: &str = "textures/environment/end_sky.png";

pub(crate) fn load_sky_textures(renderer: &mut bbb_renderer::Renderer, roots: Option<&PackRoots>) {
    let Some(roots) = roots else {
        tracing::warn!(
            "continuing without vanilla sky textures because pack roots are unavailable"
        );
        return;
    };
    if let Err(err) = try_load_sky_textures(renderer, roots) {
        tracing::warn!(?err, "continuing without vanilla sky textures");
    }
}

fn try_load_sky_textures(renderer: &mut bbb_renderer::Renderer, roots: &PackRoots) -> Result<()> {
    let image = load_end_sky_image(roots)?;
    renderer.upload_end_sky_texture(image.width, image.height, &image.rgba)?;
    tracing::info!("loaded vanilla End sky texture");
    Ok(())
}

fn load_end_sky_image(roots: &PackRoots) -> Result<SpriteImage> {
    let location = ResourceLocation::parse(END_SKY_TEXTURE)?;
    let resource = roots
        .resource_stack()
        .get_resource(&location)
        .with_context(|| format!("missing End sky texture minecraft:{END_SKY_TEXTURE}"))?;
    SpriteImage::from_png_file("minecraft:textures/environment/end_sky", resource.path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_vanilla_end_sky_texture_from_resource_stack() {
        let temp = unique_temp_dir("bbb-end-sky-texture");
        let sources = temp.join("sources").join("26.1");
        let texture_path = sources
            .join("assets")
            .join("minecraft")
            .join(END_SKY_TEXTURE);
        std::fs::create_dir_all(texture_path.parent().unwrap()).unwrap();
        write_png(&texture_path, 8, 8);
        let roots = PackRoots {
            mc_code_root: temp.clone(),
            sources_dir: sources,
            assets_dir: temp.join("unused-assets"),
            generated_assets_dir: None,
            resource_pack_dirs: Vec::new(),
        };

        let image = load_end_sky_image(&roots).unwrap();

        assert_eq!(image.id, "minecraft:textures/environment/end_sky");
        assert_eq!((image.width, image.height), (8, 8));
        assert_eq!(image.rgba.len(), 8 * 8 * 4);
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
