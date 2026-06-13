use std::path::Path;

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
        let reader =
            ImageReader::open(path).with_context(|| format!("open png {}", path.display()))?;
        let reader = reader
            .with_guessed_format()
            .with_context(|| format!("guess image format {}", path.display()))?;
        let format = reader
            .format()
            .ok_or_else(|| anyhow::anyhow!("missing image format for {}", path.display()))?;
        if format != image::ImageFormat::Png {
            bail!("sprite source {} is not a PNG", path.display());
        }
        let (width, height) = reader
            .into_dimensions()
            .with_context(|| format!("read png dimensions {}", path.display()))?;
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
        let reader =
            ImageReader::open(path).with_context(|| format!("open png {}", path.display()))?;
        let reader = reader
            .with_guessed_format()
            .with_context(|| format!("guess image format {}", path.display()))?;
        let format = reader
            .format()
            .ok_or_else(|| anyhow::anyhow!("missing image format for {}", path.display()))?;
        if format != image::ImageFormat::Png {
            bail!("sprite image {} is not a PNG", path.display());
        }
        let rgba = reader
            .decode()
            .with_context(|| format!("decode png {}", path.display()))?
            .into_rgba8();
        let (width, height) = rgba.dimensions();
        Self::new(id, width, height, rgba.into_raw())
    }

    pub(crate) fn source(&self) -> SpriteSource {
        SpriteSource::new(self.id.clone(), self.width, self.height)
    }
}
