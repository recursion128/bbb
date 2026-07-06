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
}

/// A frame read back from the GPU as tightly-packed RGBA8 rows (row padding
/// stripped, BGRA surface formats converted).
pub(super) struct ScreenshotPixels {
    pub(super) rgba: Vec<u8>,
    pub(super) width: u32,
    pub(super) height: u32,
}

#[cfg(test)]
impl ScreenshotPixels {
    /// The `[r, g, b, a]` bytes at framebuffer `(x, y)` (col, row from top-left).
    pub(super) fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        assert!(
            x < self.width && y < self.height,
            "pixel ({x}, {y}) out of bounds for {}x{} frame",
            self.width,
            self.height
        );
        let offset = ((y * self.width + x) * 4) as usize;
        [
            self.rgba[offset],
            self.rgba[offset + 1],
            self.rgba[offset + 2],
            self.rgba[offset + 3],
        ]
    }
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
        })
    }

    /// Maps the copied buffer and converts it to tightly-packed RGBA8 rows.
    /// The single home of the 256-byte padded-row and BGRA-order handling —
    /// both the screenshot file path and the offscreen readback harness
    /// consume frames through here.
    pub(super) fn read_screenshot_pixels(
        &self,
        pending: PendingScreenshot,
    ) -> Result<ScreenshotPixels> {
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

        Ok(ScreenshotPixels {
            rgba,
            width: pending.width,
            height: pending.height,
        })
    }

    pub(super) fn finish_screenshot(&self, pending: PendingScreenshot, path: &Path) -> Result<()> {
        let pixels = self.read_screenshot_pixels(pending)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        image::save_buffer(
            path,
            &pixels.rgba,
            pixels.width,
            pixels.height,
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
