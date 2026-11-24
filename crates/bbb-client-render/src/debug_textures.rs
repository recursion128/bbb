use std::path::Path;

use anyhow::Result;
#[cfg(not(target_arch = "wasm32"))]
use anyhow::{anyhow, Context};

use crate::Renderer;
#[cfg(not(target_arch = "wasm32"))]
use crate::{
    entity_models::{build_dynamic_player_skin_atlas, build_dynamic_player_texture_atlas},
    player_skin::{DynamicPlayerSkinImage, DynamicPlayerTextureImage},
};

#[cfg(not(target_arch = "wasm32"))]
const TERRAIN_ATLAS_DUMP_FILE: &str = "bbb_terrain_atlas.png";
#[cfg(not(target_arch = "wasm32"))]
const GUI_ATLAS_DUMP_FILE: &str = "bbb_gui_atlas.png";
#[cfg(not(target_arch = "wasm32"))]
const FONT_ATLAS_DUMP_FILE: &str = "bbb_font_atlas.png";
#[cfg(not(target_arch = "wasm32"))]
const PARTICLE_ATLAS_DUMP_FILE: &str = "bbb_particle_atlas.png";
#[cfg(not(target_arch = "wasm32"))]
const DYNAMIC_PLAYER_SKIN_ATLAS_DUMP_FILE: &str = "bbb_entity_dynamic_player_skins.png";
#[cfg(not(target_arch = "wasm32"))]
const DYNAMIC_PLAYER_TEXTURE_ATLAS_DUMP_FILE: &str = "bbb_entity_dynamic_player_textures.png";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DynamicTextureDumpSummary {
    pub files_written: usize,
    pub terrain_atlas: bool,
    pub gui_atlas: bool,
    pub font_atlas: bool,
    pub particle_atlas: bool,
    pub dynamic_player_skin_atlas: bool,
    pub dynamic_player_texture_atlas: bool,
}

impl Renderer {
    /// Vanilla `TextureManager.dumpAllSheets` creates the target debug directory
    /// and asks each dumpable texture to write its current sheet.
    pub fn dump_dynamic_textures(&self, target_dir: &Path) -> Result<DynamicTextureDumpSummary> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = target_dir;
            anyhow::bail!(
                "debug texture dumps (F3+S) are unavailable on wasm32 (no local filesystem or synchronous GPU readback)"
            );
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut summary = dump_dynamic_texture_images(
                target_dir,
                &self.entity_dynamic_player_skin_images,
                &self.entity_dynamic_player_texture_images,
            )?;

            let mut gpu_sheets: Vec<(&'static str, &wgpu::Texture, u32, u32)> = Vec::new();
            let terrain = &self.pack.terrain_atlas;
            if terrain.width > 1 || terrain.height > 1 {
                gpu_sheets.push((
                    TERRAIN_ATLAS_DUMP_FILE,
                    &terrain.texture,
                    terrain.width,
                    terrain.height,
                ));
            }
            if let Some(gui) = self.pack.gui_sprites.atlas_texture() {
                if gui.width > 0 && gui.height > 0 {
                    gpu_sheets.push((GUI_ATLAS_DUMP_FILE, &gui.texture, gui.width, gui.height));
                }
            }
            if let Some(font) = self.pack.hud_font_atlas.as_ref() {
                if font.gpu.width > 0 && font.gpu.height > 0 {
                    gpu_sheets.push((
                        FONT_ATLAS_DUMP_FILE,
                        &font.gpu.texture,
                        font.gpu.width,
                        font.gpu.height,
                    ));
                }
            }
            if let Some(particles) = self.pack.particle_atlas.as_ref() {
                if particles.width > 0 && particles.height > 0 {
                    gpu_sheets.push((
                        PARTICLE_ATLAS_DUMP_FILE,
                        &particles.texture,
                        particles.width,
                        particles.height,
                    ));
                }
            }

            if !gpu_sheets.is_empty() {
                let readbacks = readback_rgba8_textures(&self.device, &self.queue, &gpu_sheets)?;
                for ((file_name, _, width, height), rgba) in gpu_sheets.into_iter().zip(readbacks) {
                    write_debug_texture_png(target_dir, file_name, &rgba, width, height)?;
                    summary.files_written += 1;
                    match file_name {
                        TERRAIN_ATLAS_DUMP_FILE => summary.terrain_atlas = true,
                        GUI_ATLAS_DUMP_FILE => summary.gui_atlas = true,
                        FONT_ATLAS_DUMP_FILE => summary.font_atlas = true,
                        PARTICLE_ATLAS_DUMP_FILE => summary.particle_atlas = true,
                        _ => {}
                    }
                }
            }

            Ok(summary)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn readback_rgba8_textures(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    sheets: &[(&'static str, &wgpu::Texture, u32, u32)],
) -> Result<Vec<Vec<u8>>> {
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("bbb-debug-texture-dump-readback"),
    });
    let mut pending = Vec::with_capacity(sheets.len());
    for &(file_name, texture, width, height) in sheets {
        let unpadded_bytes_per_row = width
            .checked_mul(4)
            .ok_or_else(|| anyhow!("debug texture dump row width overflow for {file_name}"))?;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
        let buffer_size = u64::from(padded_bytes_per_row)
            .checked_mul(u64::from(height))
            .ok_or_else(|| anyhow!("debug texture dump buffer size overflow for {file_name}"))?;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bbb-debug-texture-dump-buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        pending.push((
            file_name,
            buffer,
            width,
            height,
            padded_bytes_per_row as usize,
            unpadded_bytes_per_row as usize,
        ));
    }
    queue.submit(Some(encoder.finish()));

    let mut receivers = Vec::with_capacity(pending.len());
    for (_, buffer, _, _, _, _) in &pending {
        let slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        receivers.push(rx);
    }
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .context("wait for debug texture dump GPU readback")?;

    let mut results = Vec::with_capacity(pending.len());
    for ((file_name, buffer, width, height, padded_row, unpadded_row), rx) in
        pending.into_iter().zip(receivers)
    {
        rx.recv()
            .with_context(|| format!("debug texture dump map callback dropped for {file_name}"))?
            .map_err(|err| anyhow!("map debug texture dump buffer for {file_name}: {err}"))?;
        let slice = buffer.slice(..);
        let mapped = slice
            .get_mapped_range()
            .map_err(|err| anyhow!("access mapped debug texture dump for {file_name}: {err}"))?;
        let mut rgba = Vec::with_capacity((width as usize) * (height as usize) * 4);
        for row in mapped.chunks(padded_row).take(height as usize) {
            rgba.extend_from_slice(&row[..unpadded_row]);
        }
        drop(mapped);
        buffer.unmap();
        results.push(rgba);
    }
    Ok(results)
}

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(all(test, not(target_arch = "wasm32")))]
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
                standalone_sampling: None,
                rgba: skin_rgba,
            }],
            &[DynamicPlayerTextureImage {
                handle: 9,
                size: [2, 3],
                standalone_sampling: None,
                rgba: cape_rgba,
            }],
        )
        .unwrap();

        assert_eq!(
            summary,
            DynamicTextureDumpSummary {
                files_written: 2,
                terrain_atlas: false,
                gui_atlas: false,
                font_atlas: false,
                particle_atlas: false,
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

    #[test]
    fn renderer_dump_dynamic_textures_writes_terrain_gui_font_and_particle_atlases() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
            return;
        };
        let root = unique_temp_dir("renderer-sheets");
        renderer
            .upload_terrain_texture_atlas(2, 2, &[255, 10, 20, 255].repeat(4))
            .unwrap();
        renderer
            .add_test_gui_sprite("test/icon", 2, 2, &[10, 200, 30, 255].repeat(4))
            .unwrap();
        renderer
            .upload_hud_font_atlas(
                2,
                2,
                &[40, 50, 220, 255].repeat(4),
                bbb_render_types::HudFontGlyphBook::default(),
            )
            .unwrap();
        renderer
            .upload_particle_atlas(2, 2, &[200, 150, 50, 255].repeat(4), vec![])
            .unwrap();

        let summary = renderer.dump_dynamic_textures(&root).unwrap();
        assert_eq!(
            summary,
            DynamicTextureDumpSummary {
                files_written: 4,
                terrain_atlas: true,
                gui_atlas: true,
                font_atlas: true,
                particle_atlas: true,
                dynamic_player_skin_atlas: false,
                dynamic_player_texture_atlas: false,
            }
        );
        let terrain = image::open(root.join(TERRAIN_ATLAS_DUMP_FILE))
            .unwrap()
            .into_rgba8();
        assert_eq!(terrain.dimensions(), (2, 2));
        assert_eq!(terrain.get_pixel(0, 0).0, [255, 10, 20, 255]);
        let font = image::open(root.join(FONT_ATLAS_DUMP_FILE))
            .unwrap()
            .into_rgba8();
        assert_eq!(font.dimensions(), (2, 2));
        assert_eq!(font.get_pixel(0, 0).0, [40, 50, 220, 255]);
        let particle = image::open(root.join(PARTICLE_ATLAS_DUMP_FILE))
            .unwrap()
            .into_rgba8();
        assert_eq!(particle.dimensions(), (2, 2));
        assert_eq!(particle.get_pixel(0, 0).0, [200, 150, 50, 255]);
        assert!(root.join(GUI_ATLAS_DUMP_FILE).is_file());

        fs::remove_dir_all(root).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let id = NEXT_TEMP_DIR_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("bbb-debug-texture-dump-{label}-{id}"));
        let _ = fs::remove_dir_all(&path);
        path
    }
}
