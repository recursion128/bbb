use anyhow::{Context, Result};
use bbb_pack::{PackRoots, ResourceLocation, SpriteImage};
use bbb_renderer::{CelestialTextureImage, CelestialTextureKind};

const END_SKY_TEXTURE: &str = "textures/environment/end_sky.png";
const CELESTIAL_TEXTURES: &[(CelestialTextureKind, &str, &str)] = &[
    (
        CelestialTextureKind::Sun,
        "textures/environment/celestial/sun.png",
        "minecraft:textures/environment/celestial/sun",
    ),
    (
        CelestialTextureKind::MoonFull,
        "textures/environment/celestial/moon/full_moon.png",
        "minecraft:textures/environment/celestial/moon/full_moon",
    ),
    (
        CelestialTextureKind::MoonWaningGibbous,
        "textures/environment/celestial/moon/waning_gibbous.png",
        "minecraft:textures/environment/celestial/moon/waning_gibbous",
    ),
    (
        CelestialTextureKind::MoonThirdQuarter,
        "textures/environment/celestial/moon/third_quarter.png",
        "minecraft:textures/environment/celestial/moon/third_quarter",
    ),
    (
        CelestialTextureKind::MoonWaningCrescent,
        "textures/environment/celestial/moon/waning_crescent.png",
        "minecraft:textures/environment/celestial/moon/waning_crescent",
    ),
    (
        CelestialTextureKind::MoonNew,
        "textures/environment/celestial/moon/new_moon.png",
        "minecraft:textures/environment/celestial/moon/new_moon",
    ),
    (
        CelestialTextureKind::MoonWaxingCrescent,
        "textures/environment/celestial/moon/waxing_crescent.png",
        "minecraft:textures/environment/celestial/moon/waxing_crescent",
    ),
    (
        CelestialTextureKind::MoonFirstQuarter,
        "textures/environment/celestial/moon/first_quarter.png",
        "minecraft:textures/environment/celestial/moon/first_quarter",
    ),
    (
        CelestialTextureKind::MoonWaxingGibbous,
        "textures/environment/celestial/moon/waxing_gibbous.png",
        "minecraft:textures/environment/celestial/moon/waxing_gibbous",
    ),
];

pub(crate) fn load_sky_textures(renderer: &mut bbb_renderer::Renderer, roots: Option<&PackRoots>) {
    let Some(roots) = roots else {
        tracing::warn!(
            "continuing without vanilla sky textures because pack roots are unavailable"
        );
        return;
    };
    if let Err(err) = try_load_end_sky_texture(renderer, roots) {
        tracing::warn!(?err, "continuing without vanilla End sky texture");
    }
    if let Err(err) = try_load_celestial_textures(renderer, roots) {
        tracing::warn!(
            ?err,
            "continuing without vanilla sun/moon celestial textures"
        );
    }
}

fn try_load_end_sky_texture(
    renderer: &mut bbb_renderer::Renderer,
    roots: &PackRoots,
) -> Result<()> {
    let image = load_end_sky_image(roots)?;
    renderer.upload_end_sky_texture(image.width, image.height, &image.rgba)?;
    tracing::info!("loaded vanilla End sky texture");
    Ok(())
}

fn try_load_celestial_textures(
    renderer: &mut bbb_renderer::Renderer,
    roots: &PackRoots,
) -> Result<()> {
    let celestial_images = load_celestial_images(roots)?;
    renderer.upload_celestial_textures(&celestial_images)?;
    tracing::info!("loaded vanilla sun/moon celestial textures");
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

fn load_celestial_images(roots: &PackRoots) -> Result<Vec<CelestialTextureImage>> {
    CELESTIAL_TEXTURES
        .iter()
        .map(|(kind, path, id)| load_celestial_image(roots, *kind, path, id))
        .collect()
}

fn load_celestial_image(
    roots: &PackRoots,
    kind: CelestialTextureKind,
    path: &str,
    id: &str,
) -> Result<CelestialTextureImage> {
    let location = ResourceLocation::parse(path)?;
    let resource = roots
        .resource_stack()
        .get_resource(&location)
        .with_context(|| format!("missing celestial texture minecraft:{path}"))?;
    let image = SpriteImage::from_png_file(id, resource.path)?;
    Ok(CelestialTextureImage {
        kind,
        width: image.width,
        height: image.height,
        rgba: image.rgba,
    })
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

    #[test]
    fn loads_vanilla_celestial_textures_from_resource_stack_in_phase_order() {
        let temp = unique_temp_dir("bbb-celestial-textures");
        let sources = temp.join("sources").join("26.1");
        for (_, path, _) in CELESTIAL_TEXTURES {
            let texture_path = sources.join("assets").join("minecraft").join(path);
            std::fs::create_dir_all(texture_path.parent().unwrap()).unwrap();
            write_png(&texture_path, 8, 8);
        }
        let roots = PackRoots {
            mc_code_root: temp.clone(),
            sources_dir: sources,
            assets_dir: temp.join("unused-assets"),
            generated_assets_dir: None,
            resource_pack_dirs: Vec::new(),
        };

        let images = load_celestial_images(&roots).unwrap();

        assert_eq!(images.len(), CELESTIAL_TEXTURES.len());
        assert_eq!(images[0].kind, CelestialTextureKind::Sun);
        assert_eq!(images[1].kind, CelestialTextureKind::MoonFull);
        assert_eq!(images[2].kind, CelestialTextureKind::MoonWaningGibbous);
        assert_eq!(
            images.last().map(|image| image.kind),
            Some(CelestialTextureKind::MoonWaxingGibbous)
        );
        assert!(images
            .iter()
            .all(|image| (image.width, image.height) == (8, 8)));
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
