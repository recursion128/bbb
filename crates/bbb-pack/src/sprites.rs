use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use image::ImageReader;
use serde::{Deserialize, Serialize};

use crate::rgba_len;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpriteSource {
    pub id: String,
    pub width: u32,
    pub height: u32,
}

impl SpriteSource {
    pub fn new(id: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            id: id.into(),
            width,
            height,
        }
    }

    pub fn from_png_file(id: impl Into<String>, path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let (image_width, image_height) = png_dimensions(path, "sprite source")?;
        let (width, height) = sprite_frame_size(path, image_width, image_height)?;
        Ok(Self::new(id, width, height))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpriteImage {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl SpriteImage {
    pub fn new(id: impl Into<String>, width: u32, height: u32, rgba: Vec<u8>) -> Result<Self> {
        let expected = rgba_len(width, height)?;
        if rgba.len() != expected {
            bail!(
                "sprite image has {} RGBA bytes, expected {} for {}x{}",
                rgba.len(),
                expected,
                width,
                height
            );
        }
        Ok(Self {
            id: id.into(),
            width,
            height,
            rgba,
        })
    }

    pub fn from_png_file(id: impl Into<String>, path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let reader = png_reader(path, "sprite image")
            .with_context(|| format!("open sprite {}", path.display()))?;
        let rgba = reader
            .decode()
            .with_context(|| format!("decode png {}", path.display()))?
            .into_rgba8();
        let (image_width, image_height) = rgba.dimensions();
        let (width, height) = sprite_frame_size(path, image_width, image_height)?;
        let rgba = if (width, height) == (image_width, image_height) {
            rgba.into_raw()
        } else {
            copy_first_frame_rgba(&rgba.into_raw(), image_width, width, height)?
        };
        Self::new(id, width, height, rgba)
    }

    pub(crate) fn source(&self) -> SpriteSource {
        SpriteSource::new(self.id.clone(), self.width, self.height)
    }
}

fn png_dimensions(path: &Path, label: &str) -> Result<(u32, u32)> {
    let reader = png_reader(path, label)?;
    reader
        .into_dimensions()
        .with_context(|| format!("read png dimensions {}", path.display()))
}

fn png_reader(path: &Path, label: &str) -> Result<ImageReader<std::io::BufReader<std::fs::File>>> {
    let reader = ImageReader::open(path).with_context(|| format!("open png {}", path.display()))?;
    let reader = reader
        .with_guessed_format()
        .with_context(|| format!("guess image format {}", path.display()))?;
    let format = reader
        .format()
        .ok_or_else(|| anyhow::anyhow!("missing image format for {}", path.display()))?;
    if format != image::ImageFormat::Png {
        bail!("{label} {} is not a PNG", path.display());
    }
    Ok(reader)
}

fn sprite_frame_size(path: &Path, image_width: u32, image_height: u32) -> Result<(u32, u32)> {
    if image_width == 0 || image_height == 0 {
        bail!("sprite image {} must not be empty", path.display());
    }
    let Some(animation) = read_animation_metadata(path)? else {
        return Ok((image_width, image_height));
    };

    let width = match animation.width {
        Some(width) => positive_dimension(width, "animation width", path)?,
        None if animation.height.is_some() => image_width,
        None => image_width.min(image_height),
    };
    let height = match animation.height {
        Some(height) => positive_dimension(height, "animation height", path)?,
        None if animation.width.is_some() => image_height,
        None => image_width.min(image_height),
    };

    if image_width % width != 0 || image_height % height != 0 {
        bail!(
            "image {} size {}x{} is not a multiple of animation frame size {}x{}",
            path.display(),
            image_width,
            image_height,
            width,
            height
        );
    }
    Ok((width, height))
}

fn read_animation_metadata(path: &Path) -> Result<Option<RawAnimationMetadata>> {
    let path = mcmeta_path(path);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path).with_context(|| format!("read mcmeta {}", path.display()))?;
    let metadata: RawSpriteMetadata = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse mcmeta {}", path.display()))?;
    Ok(metadata.animation)
}

fn mcmeta_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|name| format!("{}.mcmeta", name.to_string_lossy()))
        .unwrap_or_else(|| ".mcmeta".to_string());
    path.with_file_name(file_name)
}

fn positive_dimension(value: u32, name: &str, path: &Path) -> Result<u32> {
    if value == 0 {
        bail!("{name} in {} must be positive", path.display());
    }
    Ok(value)
}

fn copy_first_frame_rgba(
    rgba: &[u8],
    image_width: u32,
    frame_width: u32,
    frame_height: u32,
) -> Result<Vec<u8>> {
    let source_stride = image_width as usize * 4;
    let frame_stride = frame_width as usize * 4;
    let mut frame = Vec::with_capacity(frame_stride * frame_height as usize);
    for y in 0..frame_height as usize {
        let start = y * source_stride;
        let end = start + frame_stride;
        if end > rgba.len() {
            bail!("first animation frame exceeds source image bounds");
        }
        frame.extend_from_slice(&rgba[start..end]);
    }
    Ok(frame)
}

#[derive(Debug, Deserialize)]
struct RawSpriteMetadata {
    animation: Option<RawAnimationMetadata>,
}

#[derive(Debug, Deserialize)]
struct RawAnimationMetadata {
    width: Option<u32>,
    height: Option<u32>,
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{SpriteImage, SpriteSource};

    #[test]
    fn sprite_source_reads_png_dimensions() {
        let dir = unique_temp_dir("png-dimensions");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sprite.png");
        write_test_png(&path, 7, 11);

        let source = SpriteSource::from_png_file("test:sprite", &path).unwrap();
        assert_eq!(source, SpriteSource::new("test:sprite", 7, 11));

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sprite_source_reads_animation_frame_dimensions_from_mcmeta() {
        let dir = unique_temp_dir("animation-dimensions");
        let path = dir.join("water_still.png");
        write_test_png(&path, 16, 48);
        write_json(
            &dir.join("water_still.png.mcmeta"),
            r#"{
              "animation": {
                "frametime": 2
              }
            }"#,
        );

        let source = SpriteSource::from_png_file("minecraft:block/water_still", &path).unwrap();
        assert_eq!(
            source,
            SpriteSource::new("minecraft:block/water_still", 16, 16)
        );

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sprite_image_crops_first_animation_frame() {
        let dir = unique_temp_dir("animation-first-frame");
        let path = dir.join("arrow.png");
        write_test_rgba_png(
            &path,
            2,
            4,
            &[
                1, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255, 10, 11, 12, 255, 13, 14, 15, 255, 16, 17,
                18, 255, 19, 20, 21, 255, 22, 23, 24, 255,
            ],
        );
        write_json(
            &dir.join("arrow.png.mcmeta"),
            r#"{
              "animation": {
                "height": 2
              }
            }"#,
        );

        let image = SpriteImage::from_png_file("minecraft:hud/arrow", &path).unwrap();
        assert_eq!((image.width, image.height), (2, 2));
        assert_eq!(
            image.rgba,
            vec![1, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255, 10, 11, 12, 255]
        );

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sprite_source_rejects_non_multiple_animation_frame_size() {
        let dir = unique_temp_dir("animation-invalid");
        let path = dir.join("bad.png");
        write_test_png(&path, 5, 9);
        write_json(
            &dir.join("bad.png.mcmeta"),
            r#"{
              "animation": {
                "width": 3,
                "height": 3
              }
            }"#,
        );

        let err = SpriteSource::from_png_file("minecraft:bad", &path).unwrap_err();
        assert!(err.to_string().contains("not a multiple"));

        std::fs::remove_dir_all(dir).unwrap();
    }

    fn write_test_png(path: &Path, width: u32, height: u32) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let image = image::RgbaImage::from_pixel(width, height, image::Rgba([1, 2, 3, 255]));
        image.save(path).unwrap();
    }

    fn write_test_rgba_png(path: &Path, width: u32, height: u32, rgba: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let image = image::RgbaImage::from_raw(width, height, rgba.to_vec()).unwrap();
        image.save(path).unwrap();
    }

    fn write_json(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-pack-{name}-{}-{nonce}", std::process::id()))
    }
}
