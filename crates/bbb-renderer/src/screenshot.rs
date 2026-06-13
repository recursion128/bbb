use std::{path::Path, sync::mpsc};

use anyhow::{anyhow, bail, Context, Result};

use crate::Renderer;

pub(super) struct PendingScreenshot {
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,
    unpadded_bytes_per_row: u32,
    format: wgpu::TextureFormat,
    path: std::path::PathBuf,
}

impl Renderer {
    pub(super) fn prepare_screenshot_copy(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        path: &Path,
    ) -> Result<PendingScreenshot> {
        let width = self.config.width;
        let height = self.config.height;
        let bytes_per_pixel = 4u32;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
        let buffer_size = padded_bytes_per_row as u64 * height as u64;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bbb-native-screenshot-buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
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

        Ok(PendingScreenshot {
            buffer,
            width,
            height,
            padded_bytes_per_row,
            unpadded_bytes_per_row,
            format: self.config.format,
            path: path.to_path_buf(),
        })
    }

    pub(super) fn finish_screenshot(&self, pending: PendingScreenshot) -> Result<()> {
        let slice = pending.buffer.slice(..);
        let (tx, rx) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .context("screenshot map callback dropped")?
            .map_err(|err| anyhow!("map screenshot buffer: {err}"))?;

        let mapped = slice.get_mapped_range();
        let mut rgba = Vec::with_capacity((pending.width * pending.height * 4) as usize);
        for row in mapped
            .chunks(pending.padded_bytes_per_row as usize)
            .take(pending.height as usize)
        {
            let row = &row[..pending.unpadded_bytes_per_row as usize];
            match pending.format {
                wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
                    for px in row.chunks_exact(4) {
                        rgba.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
                    }
                }
                wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => {
                    rgba.extend_from_slice(row);
                }
                other => bail!("unsupported screenshot surface format {other:?}"),
            }
        }
        drop(mapped);
        pending.buffer.unmap();

        if let Some(parent) = pending.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        image::save_buffer(
            &pending.path,
            &rgba,
            pending.width,
            pending.height,
            image::ColorType::Rgba8,
        )?;
        Ok(())
    }
}
