use anyhow::{bail, Result};

pub struct PendingScreenshot {
    pub buffer: wgpu::Buffer,
    pub width: u32,
    pub height: u32,
    pub padded_bytes_per_row: u32,
    pub unpadded_bytes_per_row: u32,
    pub pixel_format: ScreenshotPixelFormat,
}

/// A frame read back from the GPU as tightly-packed RGBA8 rows (row padding
/// stripped, BGRA surface formats converted).
#[derive(Debug)]
pub struct ScreenshotPixels {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl ScreenshotPixels {
    /// The `[r, g, b, a]` bytes at framebuffer `(x, y)` (col, row from top-left).
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
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
pub enum ScreenshotPixelFormat {
    Bgra8,
    Rgba8,
}

impl ScreenshotPixelFormat {
    pub fn from_texture_format(format: wgpu::TextureFormat) -> Result<Self> {
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

    pub fn append_row_as_rgba(self, row: &[u8], rgba: &mut Vec<u8>) {
        match self {
            Self::Bgra8 => {
                for px in row.as_chunks::<4>().0 {
                    rgba.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
                }
            }
            Self::Rgba8 => rgba.extend_from_slice(row),
        }
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
