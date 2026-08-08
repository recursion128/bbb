use anyhow::{Context, Result};

use crate::{
    image::decode_png_file_rgba_with_budget, sprites::read_texture_metadata, PackRoots,
    ResourceLocation, SpriteTextureMetadata,
};

/// One pack-backed `ClientAsset.ResourceTexture`, decoded at its original PNG dimensions.
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceTextureImage {
    pub asset_id: String,
    pub texture_path: String,
    pub size: [u32; 2],
    pub texture_metadata: SpriteTextureMetadata,
    pub rgba: Vec<u8>,
}

impl PackRoots {
    /// Resolves a `ClientAsset.ResourceTexture` through the selected resource-pack stack.
    ///
    /// Vanilla derives the texture identifier by preserving the asset namespace and replacing its
    /// path with `textures/<asset path>.png`. The highest-priority non-filtered pack resource wins.
    pub fn load_resource_texture_image(&self, asset_id: &str) -> Result<ResourceTextureImage> {
        let asset = ResourceLocation::parse(asset_id)
            .with_context(|| format!("invalid resource texture asset id {asset_id:?}"))?;
        let texture =
            ResourceLocation::new(asset.namespace(), format!("textures/{}.png", asset.path()))?;
        let resources = self.resource_stack();
        let resource = resources
            .get_resource(&texture)
            .with_context(|| format!("missing resource texture {}", texture.id()))?;
        let path = resource.materialized_path()?.to_path_buf();
        let texture_metadata = read_texture_metadata(&path, &resources)?;
        let image = decode_png_file_rgba_with_budget(
            &path,
            "resource texture",
            &mut resources.single_image_png_decode_budget(),
        )?;
        let size = [image.width(), image.height()];

        Ok(ResourceTextureImage {
            asset_id: asset.id(),
            texture_path: texture.id(),
            size,
            texture_metadata,
            rgba: image.into_raw(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    #[test]
    fn loads_namespaced_resource_textures_with_pack_precedence_and_real_dimensions() {
        let root = unique_temp_dir("resource-texture-stack");
        let sources = root.join("sources").join("26.1");
        let overlay = root.join("overlay");
        write_png(
            &sources.join("assets/minecraft/textures/entity/cow/custom.png"),
            2,
            1,
            [1, 2, 3, 255],
        );
        std::fs::write(
            sources.join("assets/minecraft/textures/entity/cow/custom.png.mcmeta"),
            br#"{"texture":{"blur":true,"clamp":true}}"#,
        )
        .unwrap();
        write_png(
            &overlay.join("assets/minecraft/textures/entity/cow/custom.png"),
            1,
            2,
            [7, 11, 13, 255],
        );
        write_png(
            &overlay.join("assets/example/textures/animals/calf.png"),
            3,
            1,
            [17, 19, 23, 255],
        );
        std::fs::write(
            overlay.join("assets/example/textures/animals/calf.png.mcmeta"),
            br#"{"texture":{"blur":true,"clamp":true}}"#,
        )
        .unwrap();
        let roots = roots(&root, &sources, vec![overlay]);

        let overridden = roots
            .load_resource_texture_image("entity/cow/custom")
            .unwrap();
        assert_eq!(overridden.asset_id, "minecraft:entity/cow/custom");
        assert_eq!(
            overridden.texture_path,
            "minecraft:textures/entity/cow/custom.png"
        );
        assert_eq!(overridden.size, [1, 2]);
        assert_eq!(
            overridden.texture_metadata,
            SpriteTextureMetadata::default()
        );
        assert_eq!(overridden.rgba, [7, 11, 13, 255].repeat(2));

        let custom = roots
            .load_resource_texture_image("example:animals/calf")
            .unwrap();
        assert_eq!(custom.asset_id, "example:animals/calf");
        assert_eq!(custom.texture_path, "example:textures/animals/calf.png");
        assert_eq!(custom.size, [3, 1]);
        assert!(custom.texture_metadata.blur);
        assert!(custom.texture_metadata.clamp);
        assert_eq!(custom.rgba, [17, 19, 23, 255].repeat(3));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_invalid_and_non_png_resource_textures_fail_closed() {
        let root = unique_temp_dir("resource-texture-errors");
        let sources = root.join("sources").join("26.1");
        let bad = sources.join("assets/minecraft/textures/entity/bad.png");
        std::fs::create_dir_all(bad.parent().unwrap()).unwrap();
        std::fs::write(&bad, b"not a png").unwrap();
        let roots = roots(&root, &sources, Vec::new());

        let missing = roots
            .load_resource_texture_image("minecraft:entity/missing")
            .unwrap_err()
            .to_string();
        assert!(missing.contains("missing resource texture"));
        let invalid = roots
            .load_resource_texture_image("Example:entity/cow")
            .unwrap_err()
            .to_string();
        assert!(invalid.contains("invalid resource texture asset id"));
        let malformed = roots
            .load_resource_texture_image("minecraft:entity/bad")
            .unwrap_err()
            .to_string();
        assert!(malformed.contains("resource texture"));

        std::fs::remove_dir_all(root).unwrap();
    }

    fn roots(root: &Path, sources: &Path, resource_pack_dirs: Vec<PathBuf>) -> PackRoots {
        PackRoots::from_paths(
            root.to_path_buf(),
            sources.to_path_buf(),
            sources.join("assets/minecraft"),
        )
        .with_resource_pack_dirs(resource_pack_dirs)
    }

    fn write_png(path: &Path, width: u32, height: u32, color: [u8; 4]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let image = image::RgbaImage::from_pixel(width, height, image::Rgba(color));
        image.save(path).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-{label}-{}-{nanos}", std::process::id()))
    }
}
