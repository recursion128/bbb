use std::path::Path;

use anyhow::{Context, Result};

use crate::{
    entity_models::{build_dynamic_player_skin_atlas, build_dynamic_player_texture_atlas},
    player_skin::{DynamicPlayerSkinImage, DynamicPlayerTextureImage},
    Renderer,
};

const DYNAMIC_PLAYER_SKIN_ATLAS_DUMP_FILE: &str = "bbb_entity_dynamic_player_skins.png";
const DYNAMIC_PLAYER_TEXTURE_ATLAS_DUMP_FILE: &str = "bbb_entity_dynamic_player_textures.png";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DynamicTextureDumpSummary {
    pub files_written: usize,
    pub dynamic_player_skin_atlas: bool,
    pub dynamic_player_texture_atlas: bool,
}

impl Renderer {
    /// Vanilla `TextureManager.dumpAllSheets` creates the target debug directory
    /// and asks each dumpable texture to write its current sheet.
    pub fn dump_dynamic_textures(&self, target_dir: &Path) -> Result<DynamicTextureDumpSummary> {
        dump_dynamic_texture_images(
            target_dir,
            &self.entity_dynamic_player_skin_images,
            &self.entity_dynamic_player_texture_images,
        )
    }
}

fn dump_dynamic_texture_images(
    target_dir: &Path,
    dynamic_player_skin_images: &[DynamicPlayerSkinImage],
    dynamic_player_texture_images: &[DynamicPlayerTextureImage],
) -> Result<DynamicTextureDumpSummary> {
    std::fs::create_dir_all(target_dir).with_context(|| {
        format!(
            "create dynamic texture dump directory {}",
            target_dir.display()
        )
    })?;

    let mut summary = DynamicTextureDumpSummary::default();
    if !dynamic_player_skin_images.is_empty() {
        let (layout, rgba) = build_dynamic_player_skin_atlas(dynamic_player_skin_images)?;
        write_debug_texture_png(
            target_dir,
            DYNAMIC_PLAYER_SKIN_ATLAS_DUMP_FILE,
            &rgba,
            layout.width,
            layout.height,
        )?;
        summary.files_written += 1;
        summary.dynamic_player_skin_atlas = true;
    }

    if !dynamic_player_texture_images.is_empty() {
        let (layout, rgba) = build_dynamic_player_texture_atlas(dynamic_player_texture_images)?;
        write_debug_texture_png(
            target_dir,
            DYNAMIC_PLAYER_TEXTURE_ATLAS_DUMP_FILE,
            &rgba,
            layout.width,
            layout.height,
        )?;
        summary.files_written += 1;
        summary.dynamic_player_texture_atlas = true;
    }

    Ok(summary)
}

fn write_debug_texture_png(
    target_dir: &Path,
    file_name: &str,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> Result<()> {
    let path = target_dir.join(file_name);
    image::save_buffer(&path, rgba, width, height, image::ColorType::Rgba8)
        .with_context(|| format!("write dynamic texture dump {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_TEMP_DIR_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn dynamic_texture_dump_creates_target_dir_without_images() {
        let root = unique_temp_dir("empty");

        let summary = dump_dynamic_texture_images(&root, &[], &[]).unwrap();

        assert_eq!(summary, DynamicTextureDumpSummary::default());
        assert!(root.is_dir());
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dynamic_texture_dump_writes_current_dynamic_atlases() {
        let root = unique_temp_dir("atlases");
        let mut skin_rgba = vec![0u8; 64 * 64 * 4];
        skin_rgba[0..4].copy_from_slice(&[255, 0, 0, 255]);
        let mut cape_rgba = vec![0u8; 2 * 3 * 4];
        cape_rgba[0..4].copy_from_slice(&[0, 255, 0, 255]);

        let summary = dump_dynamic_texture_images(
            &root,
            &[DynamicPlayerSkinImage {
                handle: 2,
                rgba: skin_rgba,
            }],
            &[DynamicPlayerTextureImage {
                handle: 9,
                size: [2, 3],
                rgba: cape_rgba,
            }],
        )
        .unwrap();

        assert_eq!(
            summary,
            DynamicTextureDumpSummary {
                files_written: 2,
                dynamic_player_skin_atlas: true,
                dynamic_player_texture_atlas: true,
            }
        );
        let skin = image::open(root.join(DYNAMIC_PLAYER_SKIN_ATLAS_DUMP_FILE))
            .unwrap()
            .into_rgba8();
        assert_eq!(skin.dimensions(), (64, 64));
        assert_eq!(skin.get_pixel(0, 0).0, [255, 0, 0, 255]);
        let texture = image::open(root.join(DYNAMIC_PLAYER_TEXTURE_ATLAS_DUMP_FILE))
            .unwrap()
            .into_rgba8();
        assert_eq!(texture.dimensions(), (2, 3));
        assert_eq!(texture.get_pixel(0, 0).0, [0, 255, 0, 255]);

        fs::remove_dir_all(root).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let id = NEXT_TEMP_DIR_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("bbb-debug-texture-dump-{label}-{id}"));
        let _ = fs::remove_dir_all(&path);
        path
    }
}
