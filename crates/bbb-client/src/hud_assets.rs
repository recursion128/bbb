use anyhow::{Context, Result};
use bbb_client_assets::{PackRoots, PanoramaCatalog, ResourceLocation, SpriteImage};
use bbb_client_render::{
    HudLocatorStyle, StandaloneTextureSampling, GUI_TEXTURES, VIGNETTE_TEXTURE_PATH,
};

use bbb_client_render::item_model::font::HudFontAtlasImage;

pub(crate) fn load_hud_textures(
    renderer: &mut bbb_client_render::Renderer,
    resources: Option<(&PackRoots, HudFontAtlasImage)>,
) {
    let Some((roots, font_atlas)) = resources else {
        tracing::warn!("continuing without vanilla HUD sprites because pack roots are unavailable");
        return;
    };
    if let Err(err) = try_load_hud_textures(renderer, roots, font_atlas) {
        tracing::warn!(?err, "continuing without vanilla HUD sprites");
    }
}

pub(crate) fn load_panorama(renderer: &mut bbb_client_render::Renderer, roots: Option<&PackRoots>) {
    let Some(roots) = roots else {
        tracing::warn!("continuing without panorama because pack roots are unavailable");
        return;
    };
    let result = roots
        .load_panorama_catalog()
        .context("prepare startup panorama resources")
        .and_then(|catalog| upload_panorama(renderer, &catalog));
    if let Err(error) = result {
        tracing::warn!(%error, "continuing without panorama; menu background remains active");
    }
}

pub(crate) fn upload_panorama(
    renderer: &mut bbb_client_render::Renderer,
    catalog: &PanoramaCatalog,
) -> Result<()> {
    let faces = catalog.faces();
    let face_rgba = std::array::from_fn(|index| faces[index].rgba.as_slice());
    let overlay = catalog.overlay();
    renderer
        .upload_hud_panorama(
            faces[0].width,
            faces[0].height,
            face_rgba,
            overlay.width,
            overlay.height,
            &overlay.rgba,
            standalone_sampling(overlay),
        )
        .context("upload panorama GPU resources")
}

pub(crate) fn try_load_hud_textures(
    renderer: &mut bbb_client_render::Renderer,
    roots: &PackRoots,
    font_atlas: HudFontAtlasImage,
) -> Result<()> {
    renderer.upload_gui_sprites(&roots.load_atlas_texture_images("gui")?)?;
    for path in GUI_TEXTURES {
        let image = gui_texture(roots, path)?;
        renderer.upload_gui_texture(
            path,
            image.width,
            image.height,
            &image.rgba,
            standalone_sampling(&image),
        )?;
    }
    let vignette = load_vignette_image(roots)?;
    renderer.upload_hud_vignette_texture_with_sampling(
        vignette.width,
        vignette.height,
        &vignette.rgba,
        standalone_sampling(&vignette),
    )?;
    let underwater = gui_texture(roots, "textures/misc/underwater.png")?;
    renderer.upload_screen_effect_underwater_texture(
        underwater.width,
        underwater.height,
        &underwater.rgba,
    )?;
    let credits_vignette = gui_texture(roots, "textures/misc/credits_vignette.png")?;
    renderer.upload_hud_win_credits_vignette_with_sampling(
        credits_vignette.width,
        credits_vignette.height,
        &credits_vignette.rgba,
        standalone_sampling(&credits_vignette),
    )?;
    let waypoint_styles = roots.load_waypoint_style_catalog()?;
    renderer.set_hud_locator_styles(
        waypoint_styles
            .styles()
            .iter()
            .map(|(style_id, style)| {
                (
                    style_id.clone(),
                    HudLocatorStyle {
                        near_distance: style.near_distance,
                        far_distance: style.far_distance,
                        sprite_ids: style.sprite_locations.clone(),
                    },
                )
            })
            .collect(),
    );
    renderer.upload_hud_font_atlas(
        font_atlas.width,
        font_atlas.height,
        &font_atlas.rgba,
        font_atlas.fonts,
    )?;
    Ok(())
}

fn load_vignette_image(roots: &PackRoots) -> Result<SpriteImage> {
    gui_texture(roots, VIGNETTE_TEXTURE_PATH)
}

/// Extracts a HUD sprite's nine-slice scaling from its `gui.scaling` mcmeta, failing loudly if the
/// sprite is not declared `nine_slice` (the tooltip, system-toast, and tutorial-toast sprites used
/// here are).
fn gui_texture(roots: &PackRoots, path: &str) -> Result<SpriteImage> {
    let location = ResourceLocation::parse(path)?;
    let resources = roots.resource_stack();
    let resource = resources
        .get_resource(&location)
        .with_context(|| format!("missing GUI texture minecraft:{path}"))?;
    let id = format!("minecraft:{}", path.strip_suffix(".png").unwrap_or(path));
    SpriteImage::from_pack_resource(&id, &resource, &resources)
}

fn standalone_sampling(image: &SpriteImage) -> StandaloneTextureSampling {
    StandaloneTextureSampling::new(image.texture_metadata.blur, image.texture_metadata.clamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vignette_texture_loads_from_the_selected_resource_stack() {
        let temp = unique_temp_dir("bbb-vignette-texture");
        let sources = temp.join("sources").join("26.1");
        let texture_path = sources
            .join("assets")
            .join("minecraft")
            .join(VIGNETTE_TEXTURE_PATH);
        std::fs::create_dir_all(texture_path.parent().unwrap()).unwrap();
        image::RgbaImage::from_pixel(8, 4, image::Rgba([7, 11, 13, 255]))
            .save(&texture_path)
            .unwrap();
        std::fs::write(
            texture_path.with_extension("png.mcmeta"),
            r#"{"texture":{"blur":true,"clamp":false}}"#,
        )
        .unwrap();
        let roots = PackRoots::from_paths(temp.clone(), sources, temp.join("unused-assets"));

        let image = load_vignette_image(&roots).unwrap();

        assert_eq!(image.id, "minecraft:textures/misc/vignette");
        assert_eq!((image.width, image.height), (8, 4));
        assert_eq!(image.rgba.len(), 8 * 4 * 4);
        assert!(image.texture_metadata.blur);
        assert!(!image.texture_metadata.clamp);
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn spyglass_scope_texture_loads_from_the_selected_resource_stack() {
        let temp = unique_temp_dir("bbb-spyglass-scope-texture");
        let sources = temp.join("sources").join("26.1");
        let texture_path = sources
            .join("assets")
            .join("minecraft")
            .join("textures/misc/spyglass_scope.png");
        std::fs::create_dir_all(texture_path.parent().unwrap()).unwrap();
        image::RgbaImage::from_pixel(16, 8, image::Rgba([17, 19, 23, 255]))
            .save(&texture_path)
            .unwrap();
        std::fs::write(
            texture_path.with_extension("png.mcmeta"),
            r#"{"texture":{"blur":true,"clamp":true}}"#,
        )
        .unwrap();
        let roots = PackRoots::from_paths(temp.clone(), sources, temp.join("unused-assets"));

        let image = gui_texture(&roots, "textures/misc/spyglass_scope.png").unwrap();

        assert_eq!(image.id, "minecraft:textures/misc/spyglass_scope");
        assert_eq!((image.width, image.height), (16, 8));
        assert_eq!(
            standalone_sampling(&image),
            StandaloneTextureSampling::new(true, true)
        );
        assert_eq!(&image.rgba[..4], &[17, 19, 23, 255]);
        std::fs::remove_dir_all(temp).unwrap();
    }

    fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
        let nanos = bbb_client_clock::SystemTime::now()
            .duration_since(bbb_client_clock::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
    }
}
