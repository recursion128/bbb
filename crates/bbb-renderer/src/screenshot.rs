use std::{path::Path, sync::mpsc};

use anyhow::{anyhow, bail, Context, Result};

use crate::Renderer;

pub(super) struct PendingScreenshot {
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,
    unpadded_bytes_per_row: u32,
    pixel_format: ScreenshotPixelFormat,
    path: std::path::PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScreenshotPixelFormat {
    Bgra8,
    Rgba8,
}

impl ScreenshotPixelFormat {
    fn from_texture_format(format: wgpu::TextureFormat) -> Result<Self> {
        match format {
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
                Ok(Self::Bgra8)
            }
            wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => {
                Ok(Self::Rgba8)
            }
            other => bail!("unsupported screenshot surface format {other:?}"),
        }
    }

    fn append_row_as_rgba(self, row: &[u8], rgba: &mut Vec<u8>) {
        match self {
            Self::Bgra8 => {
                for px in row.chunks_exact(4) {
                    rgba.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
                }
            }
            Self::Rgba8 => rgba.extend_from_slice(row),
        }
    }
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
        let pixel_format = ScreenshotPixelFormat::from_texture_format(self.config.format)?;
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
            pixel_format,
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
            pending.pixel_format.append_row_as_rgba(row, &mut rgba);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screenshot_readback_accepts_renderer_chosen_surface_formats() {
        assert_eq!(
            ScreenshotPixelFormat::from_texture_format(wgpu::TextureFormat::Bgra8Unorm).unwrap(),
            ScreenshotPixelFormat::Bgra8
        );
        assert_eq!(
            ScreenshotPixelFormat::from_texture_format(wgpu::TextureFormat::Bgra8UnormSrgb)
                .unwrap(),
            ScreenshotPixelFormat::Bgra8
        );
        assert_eq!(
            ScreenshotPixelFormat::from_texture_format(wgpu::TextureFormat::Rgba8Unorm).unwrap(),
            ScreenshotPixelFormat::Rgba8
        );
        assert_eq!(
            ScreenshotPixelFormat::from_texture_format(wgpu::TextureFormat::Rgba8UnormSrgb)
                .unwrap(),
            ScreenshotPixelFormat::Rgba8
        );
    }

    #[test]
    fn screenshot_readback_rejects_non_color_surface_formats() {
        let err = ScreenshotPixelFormat::from_texture_format(wgpu::TextureFormat::Depth24Plus)
            .unwrap_err();
        assert!(err
            .to_string()
            .contains("unsupported screenshot surface format"));
    }

    #[test]
    fn screenshot_readback_converts_bgra_rows_to_rgba() {
        let mut rgba = Vec::new();
        ScreenshotPixelFormat::Bgra8.append_row_as_rgba(
            &[
                10, 20, 30, 40, //
                50, 60, 70, 80,
            ],
            &mut rgba,
        );

        assert_eq!(
            rgba,
            vec![
                30, 20, 10, 40, //
                70, 60, 50, 80,
            ]
        );
    }

    #[test]
    fn screenshot_readback_preserves_rgba_rows() {
        let row = vec![
            1, 2, 3, 4, //
            5, 6, 7, 8,
        ];
        let mut rgba = Vec::new();
        ScreenshotPixelFormat::Rgba8.append_row_as_rgba(&row, &mut rgba);

        assert_eq!(rgba, row);
    }
}
