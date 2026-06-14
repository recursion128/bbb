use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use image::ImageReader;
use serde::{Deserialize, Serialize};

use crate::rgba_len;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpriteSource {
    pub id: String,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub animation: Option<SpriteAnimation>,
    #[serde(default)]
    pub texture_metadata: SpriteTextureMetadata,
    #[serde(default)]
    pub gui_metadata: SpriteGuiMetadata,
}

impl SpriteSource {
    pub fn new(id: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            id: id.into(),
            width,
            height,
            animation: None,
            texture_metadata: SpriteTextureMetadata::default(),
            gui_metadata: SpriteGuiMetadata::default(),
        }
    }

    pub fn from_png_file(id: impl Into<String>, path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let (image_width, image_height) = png_dimensions(path, "sprite source")?;
        let metadata = read_sprite_metadata(path)?;
        let texture_metadata = metadata.texture.unwrap_or_default().into_metadata();
        let gui_metadata = metadata.gui.unwrap_or_default().into_metadata(path)?;
        let (width, height, animation) =
            sprite_frame_metadata(path, image_width, image_height, metadata.animation)?;
        Ok(Self {
            id: id.into(),
            width,
            height,
            animation,
            texture_metadata,
            gui_metadata,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpriteImage {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub transparency: SpriteTransparency,
    #[serde(default)]
    pub animation: Option<SpriteAnimation>,
    #[serde(default)]
    pub texture_metadata: SpriteTextureMetadata,
    #[serde(default)]
    pub gui_metadata: SpriteGuiMetadata,
    #[serde(default)]
    pub animation_frames_rgba: Vec<Vec<u8>>,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpriteTextureMetadata {
    pub blur: bool,
    pub clamp: bool,
    pub mipmap_strategy: SpriteMipmapStrategy,
    pub alpha_cutoff_bias: f32,
}

impl Default for SpriteTextureMetadata {
    fn default() -> Self {
        Self {
            blur: false,
            clamp: false,
            mipmap_strategy: SpriteMipmapStrategy::Auto,
            alpha_cutoff_bias: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpriteMipmapStrategy {
    #[default]
    Auto,
    Mean,
    Cutout,
    StrictCutout,
    DarkCutout,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpriteGuiMetadata {
    #[serde(default)]
    pub scaling: SpriteGuiScaling,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SpriteGuiScaling {
    #[default]
    Stretch,
    Tile {
        width: u32,
        height: u32,
    },
    NineSlice {
        width: u32,
        height: u32,
        border: SpriteNineSliceBorder,
        #[serde(default)]
        stretch_inner: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SpriteNineSliceBorder {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

impl SpriteNineSliceBorder {
    pub fn uniform(size: u32) -> Self {
        Self {
            left: size,
            top: size,
            right: size,
            bottom: size,
        }
    }
}

impl<'de> Deserialize<'de> for SpriteNineSliceBorder {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        RawNineSliceBorder::deserialize(deserializer).map(RawNineSliceBorder::into_border_unchecked)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpriteAnimation {
    pub frame_count: u32,
    pub default_frame_time: u32,
    pub interpolate: bool,
    pub frames: Vec<SpriteAnimationFrame>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpriteAnimationFrame {
    pub index: u32,
    pub time: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteAnimationFrameTick {
    pub frame_index: u32,
    pub next_frame_index: u32,
    pub frame_progress: f32,
}

impl SpriteAnimation {
    pub fn frame_index_at_tick(&self, tick: u64) -> Option<u32> {
        self.frame_at_tick(tick).map(|frame| frame.frame_index)
    }

    pub fn frame_at_tick(&self, tick: u64) -> Option<SpriteAnimationFrameTick> {
        if self.frames.is_empty() {
            return None;
        }
        let total_duration = self.frames.iter().try_fold(0u64, |total, frame| {
            total.checked_add(u64::from(frame.time))
        })?;
        if total_duration == 0 {
            return None;
        }

        let mut remaining = tick % total_duration;
        for (position, frame) in self.frames.iter().enumerate() {
            let duration = u64::from(frame.time);
            if remaining < duration {
                let next = self.frames[(position + 1) % self.frames.len()].index;
                return Some(SpriteAnimationFrameTick {
                    frame_index: frame.index,
                    next_frame_index: next,
                    frame_progress: remaining as f32 / duration as f32,
                });
            }
            remaining -= duration;
        }
        self.frames.last().map(|frame| SpriteAnimationFrameTick {
            frame_index: frame.index,
            next_frame_index: self.frames[0].index,
            frame_progress: 0.0,
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpriteTransparency {
    pub has_transparent: bool,
    pub has_translucent: bool,
}

impl SpriteTransparency {
    pub fn from_rgba(rgba: &[u8]) -> Self {
        let mut transparency = Self::default();
        for alpha in rgba.chunks_exact(4).map(|pixel| pixel[3]) {
            if alpha == 0 {
                transparency.has_transparent = true;
            } else if alpha != 255 {
                transparency.has_translucent = true;
            }
        }
        transparency
    }

    pub fn or(self, other: Self) -> Self {
        Self {
            has_transparent: self.has_transparent || other.has_transparent,
            has_translucent: self.has_translucent || other.has_translucent,
        }
    }
}

impl SpriteImage {
    pub fn new(id: impl Into<String>, width: u32, height: u32, rgba: Vec<u8>) -> Result<Self> {
        let transparency = SpriteTransparency::from_rgba(&rgba);
        Self::new_with_transparency(id, width, height, rgba, transparency, None)
    }

    fn new_with_transparency(
        id: impl Into<String>,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
        transparency: SpriteTransparency,
        animation: Option<SpriteAnimation>,
    ) -> Result<Self> {
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
            transparency,
            animation,
            texture_metadata: SpriteTextureMetadata::default(),
            gui_metadata: SpriteGuiMetadata::default(),
            animation_frames_rgba: Vec::new(),
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
        let metadata = read_sprite_metadata(path)?;
        let texture_metadata = metadata.texture.unwrap_or_default().into_metadata();
        let gui_metadata = metadata.gui.unwrap_or_default().into_metadata(path)?;
        let (width, height, animation) =
            sprite_frame_metadata(path, image_width, image_height, metadata.animation)?;
        let source_rgba = rgba.into_raw();
        let transparency = SpriteTransparency::from_rgba(&source_rgba);
        let animation_frames_rgba = match animation.as_ref() {
            Some(animation) => {
                copy_animation_frames_rgba(&source_rgba, image_width, width, height, animation)?
            }
            None => Vec::new(),
        };
        let rgba = if (width, height) == (image_width, image_height) {
            source_rgba
        } else {
            copy_first_frame_rgba(&source_rgba, image_width, width, height)?
        };
        let mut image =
            Self::new_with_transparency(id, width, height, rgba, transparency, animation)?;
        image.texture_metadata = texture_metadata;
        image.gui_metadata = gui_metadata;
        image.animation_frames_rgba = animation_frames_rgba;
        Ok(image)
    }

    pub(crate) fn source(&self) -> SpriteSource {
        SpriteSource {
            id: self.id.clone(),
            width: self.width,
            height: self.height,
            animation: self.animation.clone(),
            texture_metadata: self.texture_metadata,
            gui_metadata: self.gui_metadata,
        }
    }

    pub fn frame_rgba(&self, frame_index: u32) -> Option<&[u8]> {
        if self.animation.is_some() {
            return self
                .animation_frames_rgba
                .get(frame_index as usize)
                .map(Vec::as_slice)
                .or_else(|| {
                    (frame_index == 0 && self.animation_frames_rgba.is_empty())
                        .then_some(self.rgba.as_slice())
                });
        }
        (frame_index == 0).then_some(self.rgba.as_slice())
    }

    pub fn frame_rgba_at_tick(&self, tick: u64) -> Result<Cow<'_, [u8]>> {
        let Some(animation) = self.animation.as_ref() else {
            return Ok(Cow::Borrowed(self.rgba.as_slice()));
        };
        let frame = animation
            .frame_at_tick(tick)
            .ok_or_else(|| anyhow::anyhow!("animated sprite {} has no frames", self.id))?;
        let current = self.frame_rgba(frame.frame_index).ok_or_else(|| {
            anyhow::anyhow!(
                "animated sprite {} missing frame {}",
                self.id,
                frame.frame_index
            )
        })?;
        if !animation.interpolate || frame.frame_progress <= 0.0 {
            return Ok(Cow::Borrowed(current));
        }
        let next = self.frame_rgba(frame.next_frame_index).ok_or_else(|| {
            anyhow::anyhow!(
                "animated sprite {} missing frame {}",
                self.id,
                frame.next_frame_index
            )
        })?;
        Ok(Cow::Owned(interpolate_rgba(
            current,
            next,
            frame.frame_progress,
        )?))
    }
}

fn interpolate_rgba(current: &[u8], next: &[u8], progress: f32) -> Result<Vec<u8>> {
    if current.len() != next.len() {
        bail!(
            "animated sprite frame length mismatch: {} bytes vs {} bytes",
            current.len(),
            next.len()
        );
    }
    let progress = progress.clamp(0.0, 1.0);
    Ok(current
        .iter()
        .zip(next)
        .map(|(current, next)| {
            let current = f32::from(*current);
            let next = f32::from(*next);
            (current + (next - current) * progress).round() as u8
        })
        .collect())
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

fn sprite_frame_metadata(
    path: &Path,
    image_width: u32,
    image_height: u32,
    animation: Option<RawAnimationMetadata>,
) -> Result<(u32, u32, Option<SpriteAnimation>)> {
    if image_width == 0 || image_height == 0 {
        bail!("sprite image {} must not be empty", path.display());
    }
    let Some(animation) = animation else {
        return Ok((image_width, image_height, None));
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
    let frame_count = (image_width / width)
        .checked_mul(image_height / height)
        .ok_or_else(|| anyhow::anyhow!("animation frame count overflow in {}", path.display()))?;
    let animation = sprite_animation(path, animation, frame_count)?;
    Ok((width, height, Some(animation)))
}

fn read_sprite_metadata(path: &Path) -> Result<RawSpriteMetadata> {
    let path = mcmeta_path(path);
    if !path.exists() {
        return Ok(RawSpriteMetadata::default());
    }
    let bytes = std::fs::read(&path).with_context(|| format!("read mcmeta {}", path.display()))?;
    let metadata: RawSpriteMetadata = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse mcmeta {}", path.display()))?;
    Ok(metadata)
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

fn sprite_animation(
    path: &Path,
    metadata: RawAnimationMetadata,
    frame_count: u32,
) -> Result<SpriteAnimation> {
    let default_frame_time =
        positive_dimension(metadata.frametime.unwrap_or(1), "animation frametime", path)?;
    let frames = match metadata.frames {
        Some(frames) => frames
            .into_iter()
            .map(|frame| frame.into_frame(default_frame_time, path))
            .collect::<Result<Vec<_>>>()?,
        None => (0..frame_count)
            .map(|index| SpriteAnimationFrame {
                index,
                time: default_frame_time,
            })
            .collect(),
    };
    if frames.is_empty() {
        bail!(
            "animation in {} must contain at least one frame",
            path.display()
        );
    }
    for frame in &frames {
        if frame.index >= frame_count {
            bail!(
                "animation frame {} in {} exceeds frame count {}",
                frame.index,
                path.display(),
                frame_count
            );
        }
    }
    Ok(SpriteAnimation {
        frame_count,
        default_frame_time,
        interpolate: metadata.interpolate.unwrap_or(false),
        frames,
    })
}

fn copy_first_frame_rgba(
    rgba: &[u8],
    image_width: u32,
    frame_width: u32,
    frame_height: u32,
) -> Result<Vec<u8>> {
    copy_frame_rgba(rgba, image_width, frame_width, frame_height, 0)
}

fn copy_animation_frames_rgba(
    rgba: &[u8],
    image_width: u32,
    frame_width: u32,
    frame_height: u32,
    animation: &SpriteAnimation,
) -> Result<Vec<Vec<u8>>> {
    (0..animation.frame_count)
        .map(|frame| copy_frame_rgba(rgba, image_width, frame_width, frame_height, frame))
        .collect()
}

fn copy_frame_rgba(
    rgba: &[u8],
    image_width: u32,
    frame_width: u32,
    frame_height: u32,
    frame_index: u32,
) -> Result<Vec<u8>> {
    let frames_per_row = image_width
        .checked_div(frame_width)
        .ok_or_else(|| anyhow::anyhow!("invalid animation frame width"))?;
    if frames_per_row == 0 {
        bail!("invalid animation frame row size");
    }
    let frame_x = (frame_index % frames_per_row)
        .checked_mul(frame_width)
        .ok_or_else(|| anyhow::anyhow!("animation frame x offset overflow"))?;
    let frame_y = (frame_index / frames_per_row)
        .checked_mul(frame_height)
        .ok_or_else(|| anyhow::anyhow!("animation frame y offset overflow"))?;
    let source_stride = image_width as usize * 4;
    let frame_stride = frame_width as usize * 4;
    let mut frame = Vec::with_capacity(frame_stride * frame_height as usize);
    for y in 0..frame_height as usize {
        let start = (frame_y as usize + y)
            .checked_mul(source_stride)
            .and_then(|row| row.checked_add(frame_x as usize * 4))
            .ok_or_else(|| anyhow::anyhow!("animation frame offset overflow"))?;
        let end = start + frame_stride;
        if end > rgba.len() {
            bail!("animation frame exceeds source image bounds");
        }
        frame.extend_from_slice(&rgba[start..end]);
    }
    Ok(frame)
}

#[derive(Debug, Default, Deserialize)]
struct RawSpriteMetadata {
    animation: Option<RawAnimationMetadata>,
    texture: Option<RawTextureMetadata>,
    gui: Option<RawGuiMetadata>,
}

#[derive(Debug, Deserialize)]
struct RawAnimationMetadata {
    width: Option<u32>,
    height: Option<u32>,
    frametime: Option<u32>,
    interpolate: Option<bool>,
    frames: Option<Vec<RawAnimationFrame>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawAnimationFrame {
    Index(u32),
    Object { index: u32, time: Option<u32> },
}

#[derive(Debug, Default, Deserialize)]
struct RawTextureMetadata {
    blur: Option<bool>,
    clamp: Option<bool>,
    mipmap_strategy: Option<SpriteMipmapStrategy>,
    alpha_cutoff_bias: Option<f32>,
}

impl RawTextureMetadata {
    fn into_metadata(self) -> SpriteTextureMetadata {
        SpriteTextureMetadata {
            blur: self.blur.unwrap_or(false),
            clamp: self.clamp.unwrap_or(false),
            mipmap_strategy: self.mipmap_strategy.unwrap_or_default(),
            alpha_cutoff_bias: self.alpha_cutoff_bias.unwrap_or(0.0),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct RawGuiMetadata {
    scaling: Option<RawGuiScaling>,
}

impl RawGuiMetadata {
    fn into_metadata(self, path: &Path) -> Result<SpriteGuiMetadata> {
        Ok(SpriteGuiMetadata {
            scaling: self
                .scaling
                .map(|scaling| scaling.into_scaling(path))
                .transpose()?
                .unwrap_or_default(),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RawGuiScaling {
    Stretch,
    Tile {
        width: u32,
        height: u32,
    },
    NineSlice {
        width: u32,
        height: u32,
        border: RawNineSliceBorder,
        stretch_inner: Option<bool>,
    },
}

impl RawGuiScaling {
    fn into_scaling(self, path: &Path) -> Result<SpriteGuiScaling> {
        match self {
            Self::Stretch => Ok(SpriteGuiScaling::Stretch),
            Self::Tile { width, height } => Ok(SpriteGuiScaling::Tile {
                width: positive_dimension(width, "gui tile width", path)?,
                height: positive_dimension(height, "gui tile height", path)?,
            }),
            Self::NineSlice {
                width,
                height,
                border,
                stretch_inner,
            } => {
                let width = positive_dimension(width, "gui nine-slice width", path)?;
                let height = positive_dimension(height, "gui nine-slice height", path)?;
                let border = border.into_border(path)?;
                let horizontal_border = border
                    .left
                    .checked_add(border.right)
                    .ok_or_else(|| anyhow::anyhow!("gui nine-slice horizontal border overflow"))?;
                if horizontal_border >= width {
                    bail!(
                        "gui nine-slice in {} has no horizontal center slice: {} + {} >= {}",
                        path.display(),
                        border.left,
                        border.right,
                        width
                    );
                }
                let vertical_border = border
                    .top
                    .checked_add(border.bottom)
                    .ok_or_else(|| anyhow::anyhow!("gui nine-slice vertical border overflow"))?;
                if vertical_border >= height {
                    bail!(
                        "gui nine-slice in {} has no vertical center slice: {} + {} >= {}",
                        path.display(),
                        border.top,
                        border.bottom,
                        height
                    );
                }
                Ok(SpriteGuiScaling::NineSlice {
                    width,
                    height,
                    border,
                    stretch_inner: stretch_inner.unwrap_or(false),
                })
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawNineSliceBorder {
    Uniform(u32),
    Sides {
        left: u32,
        top: u32,
        right: u32,
        bottom: u32,
    },
}

impl RawNineSliceBorder {
    fn into_border_unchecked(self) -> SpriteNineSliceBorder {
        match self {
            Self::Uniform(size) => SpriteNineSliceBorder::uniform(size),
            Self::Sides {
                left,
                top,
                right,
                bottom,
            } => SpriteNineSliceBorder {
                left,
                top,
                right,
                bottom,
            },
        }
    }

    fn into_border(self, path: &Path) -> Result<SpriteNineSliceBorder> {
        match self {
            Self::Uniform(size) => Ok(SpriteNineSliceBorder::uniform(positive_dimension(
                size,
                "gui nine-slice border",
                path,
            )?)),
            Self::Sides {
                left,
                top,
                right,
                bottom,
            } => Ok(SpriteNineSliceBorder {
                left,
                top,
                right,
                bottom,
            }),
        }
    }
}

impl RawAnimationFrame {
    fn into_frame(self, default_frame_time: u32, path: &Path) -> Result<SpriteAnimationFrame> {
        let (index, time) = match self {
            Self::Index(index) => (index, default_frame_time),
            Self::Object { index, time } => (
                index,
                positive_dimension(
                    time.unwrap_or(default_frame_time),
                    "animation frame time",
                    path,
                )?,
            ),
        };
        Ok(SpriteAnimationFrame { index, time })
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        SpriteAnimation, SpriteAnimationFrame, SpriteAnimationFrameTick, SpriteGuiMetadata,
        SpriteGuiScaling, SpriteImage, SpriteMipmapStrategy, SpriteNineSliceBorder, SpriteSource,
        SpriteTextureMetadata,
    };

    #[test]
    fn sprite_source_reads_png_dimensions() {
        let dir = unique_temp_dir("png-dimensions");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sprite.png");
        write_test_png(&path, 7, 11);

        let source = SpriteSource::from_png_file("test:sprite", &path).unwrap();
        assert_eq!(source, SpriteSource::new("test:sprite", 7, 11));
        assert_eq!(source.texture_metadata, SpriteTextureMetadata::default());
        assert_eq!(source.gui_metadata, SpriteGuiMetadata::default());

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sprite_source_reads_gui_nine_slice_metadata_from_mcmeta() {
        let dir = unique_temp_dir("gui-nine-slice-source");
        let path = dir.join("button.png");
        write_test_png(&path, 200, 20);
        write_json(
            &dir.join("button.png.mcmeta"),
            r#"{
              "gui": {
                "scaling": {
                  "type": "nine_slice",
                  "width": 200,
                  "height": 20,
                  "border": 3
                }
              }
            }"#,
        );

        let source = SpriteSource::from_png_file("minecraft:widget/button", &path).unwrap();

        assert_eq!(
            source.gui_metadata,
            SpriteGuiMetadata {
                scaling: SpriteGuiScaling::NineSlice {
                    width: 200,
                    height: 20,
                    border: SpriteNineSliceBorder::uniform(3),
                    stretch_inner: false,
                },
            }
        );

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sprite_source_rejects_gui_nine_slice_without_center() {
        let dir = unique_temp_dir("gui-nine-slice-invalid");
        let path = dir.join("bad_button.png");
        write_test_png(&path, 10, 10);
        write_json(
            &dir.join("bad_button.png.mcmeta"),
            r#"{
              "gui": {
                "scaling": {
                  "type": "nine_slice",
                  "width": 10,
                  "height": 10,
                  "border": {
                    "left": 5,
                    "right": 5,
                    "top": 1,
                    "bottom": 1
                  }
                }
              }
            }"#,
        );

        let err = SpriteSource::from_png_file("minecraft:widget/bad_button", &path).unwrap_err();

        assert!(err.to_string().contains("no horizontal center slice"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sprite_source_deserializes_without_animation_metadata() {
        let source: SpriteSource =
            serde_json::from_str(r#"{"id":"minecraft:block/stone","width":16,"height":16}"#)
                .unwrap();

        assert_eq!(source, SpriteSource::new("minecraft:block/stone", 16, 16));
    }

    #[test]
    fn sprite_source_reads_texture_metadata_from_mcmeta() {
        let dir = unique_temp_dir("texture-metadata-source");
        let path = dir.join("torchflower.png");
        write_test_png(&path, 16, 16);
        write_json(
            &dir.join("torchflower.png.mcmeta"),
            r#"{
              "texture": {
                "blur": true,
                "clamp": true,
                "mipmap_strategy": "strict_cutout",
                "alpha_cutoff_bias": 0.125
              }
            }"#,
        );

        let source = SpriteSource::from_png_file("minecraft:block/torchflower", &path).unwrap();

        assert_eq!(
            source.texture_metadata,
            SpriteTextureMetadata {
                blur: true,
                clamp: true,
                mipmap_strategy: SpriteMipmapStrategy::StrictCutout,
                alpha_cutoff_bias: 0.125,
            }
        );

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
            SpriteSource {
                id: "minecraft:block/water_still".to_string(),
                width: 16,
                height: 16,
                animation: Some(SpriteAnimation {
                    frame_count: 3,
                    default_frame_time: 2,
                    interpolate: false,
                    frames: vec![
                        SpriteAnimationFrame { index: 0, time: 2 },
                        SpriteAnimationFrame { index: 1, time: 2 },
                        SpriteAnimationFrame { index: 2, time: 2 },
                    ],
                }),
                texture_metadata: SpriteTextureMetadata::default(),
                gui_metadata: SpriteGuiMetadata::default(),
            }
        );

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sprite_source_reads_explicit_animation_frames() {
        let dir = unique_temp_dir("animation-explicit-frames");
        let path = dir.join("locator_bar_arrow_down.png");
        write_test_png(&path, 7, 10);
        write_json(
            &dir.join("locator_bar_arrow_down.png.mcmeta"),
            r#"{
              "animation": {
                "height": 5,
                "frames": [
                  { "index": 0, "time": 10 },
                  { "index": 1, "time": 4 }
                ]
              }
            }"#,
        );

        let source =
            SpriteSource::from_png_file("minecraft:hud/locator_bar_arrow_down", &path).unwrap();
        assert_eq!((source.width, source.height), (7, 5));
        assert_eq!(
            source.animation,
            Some(SpriteAnimation {
                frame_count: 2,
                default_frame_time: 1,
                interpolate: false,
                frames: vec![
                    SpriteAnimationFrame { index: 0, time: 10 },
                    SpriteAnimationFrame { index: 1, time: 4 },
                ],
            })
        );

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sprite_animation_selects_frame_by_tick_duration() {
        let animation = SpriteAnimation {
            frame_count: 2,
            default_frame_time: 1,
            interpolate: false,
            frames: vec![
                SpriteAnimationFrame { index: 0, time: 2 },
                SpriteAnimationFrame { index: 1, time: 1 },
            ],
        };

        assert_eq!(animation.frame_index_at_tick(0), Some(0));
        assert_eq!(animation.frame_index_at_tick(1), Some(0));
        assert_eq!(animation.frame_index_at_tick(2), Some(1));
        assert_eq!(animation.frame_index_at_tick(3), Some(0));
    }

    #[test]
    fn sprite_animation_reports_frame_progress_for_interpolation() {
        let animation = SpriteAnimation {
            frame_count: 2,
            default_frame_time: 1,
            interpolate: true,
            frames: vec![
                SpriteAnimationFrame { index: 0, time: 4 },
                SpriteAnimationFrame { index: 1, time: 2 },
            ],
        };

        assert_eq!(
            animation.frame_at_tick(0),
            Some(SpriteAnimationFrameTick {
                frame_index: 0,
                next_frame_index: 1,
                frame_progress: 0.0,
            })
        );
        assert_eq!(
            animation.frame_at_tick(1),
            Some(SpriteAnimationFrameTick {
                frame_index: 0,
                next_frame_index: 1,
                frame_progress: 0.25,
            })
        );
        assert_eq!(
            animation.frame_at_tick(3),
            Some(SpriteAnimationFrameTick {
                frame_index: 0,
                next_frame_index: 1,
                frame_progress: 0.75,
            })
        );
        assert_eq!(
            animation.frame_at_tick(4),
            Some(SpriteAnimationFrameTick {
                frame_index: 1,
                next_frame_index: 0,
                frame_progress: 0.0,
            })
        );
        assert_eq!(
            animation.frame_at_tick(5),
            Some(SpriteAnimationFrameTick {
                frame_index: 1,
                next_frame_index: 0,
                frame_progress: 0.5,
            })
        );
        assert_eq!(animation.frame_index_at_tick(6), Some(0));
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
                18, 127, 19, 20, 21, 255, 22, 23, 24, 255,
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
        assert_eq!(image.frame_rgba(0), Some(image.rgba.as_slice()));
        assert_eq!(
            image.frame_rgba(1),
            Some(
                vec![13, 14, 15, 255, 16, 17, 18, 127, 19, 20, 21, 255, 22, 23, 24, 255,]
                    .as_slice()
            )
        );
        assert_eq!(image.frame_rgba(2), None);
        assert!(image.transparency.has_translucent);
        assert_eq!(
            image.animation,
            Some(SpriteAnimation {
                frame_count: 2,
                default_frame_time: 1,
                interpolate: false,
                frames: vec![
                    SpriteAnimationFrame { index: 0, time: 1 },
                    SpriteAnimationFrame { index: 1, time: 1 },
                ],
            })
        );

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sprite_image_reads_animation_frames_in_row_major_order() {
        let dir = unique_temp_dir("animation-row-major-frames");
        let path = dir.join("grid.png");
        write_test_rgba_png(
            &path,
            4,
            4,
            &[
                1, 0, 0, 255, 2, 0, 0, 255, 3, 0, 0, 255, 4, 0, 0, 255, 5, 0, 0, 255, 6, 0, 0, 255,
                7, 0, 0, 255, 8, 0, 0, 255, 9, 0, 0, 255, 10, 0, 0, 255, 11, 0, 0, 255, 12, 0, 0,
                255, 13, 0, 0, 255, 14, 0, 0, 255, 15, 0, 0, 255, 16, 0, 0, 255,
            ],
        );
        write_json(
            &dir.join("grid.png.mcmeta"),
            r#"{
              "animation": {
                "width": 2,
                "height": 2
              }
            }"#,
        );

        let image = SpriteImage::from_png_file("minecraft:block/grid", &path).unwrap();

        assert_eq!(
            image.frame_rgba(2),
            Some(vec![9, 0, 0, 255, 10, 0, 0, 255, 13, 0, 0, 255, 14, 0, 0, 255].as_slice())
        );

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sprite_image_records_interpolated_animation_metadata() {
        let dir = unique_temp_dir("animation-interpolate");
        let path = dir.join("sculk.png");
        write_test_png(&path, 2, 4);
        write_json(
            &dir.join("sculk.png.mcmeta"),
            r#"{
              "animation": {
                "frametime": 20,
                "interpolate": true
              }
            }"#,
        );

        let image = SpriteImage::from_png_file("minecraft:block/sculk", &path).unwrap();
        assert_eq!(
            image.animation,
            Some(SpriteAnimation {
                frame_count: 2,
                default_frame_time: 20,
                interpolate: true,
                frames: vec![
                    SpriteAnimationFrame { index: 0, time: 20 },
                    SpriteAnimationFrame { index: 1, time: 20 },
                ],
            })
        );

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sprite_image_records_texture_metadata() {
        let dir = unique_temp_dir("texture-metadata-image");
        let path = dir.join("glass.png");
        write_test_png(&path, 16, 16);
        write_json(
            &dir.join("glass.png.mcmeta"),
            r#"{
              "texture": {
                "mipmap_strategy": "mean"
              }
            }"#,
        );

        let image = SpriteImage::from_png_file("minecraft:block/glass", &path).unwrap();

        assert_eq!(
            image.texture_metadata,
            SpriteTextureMetadata {
                blur: false,
                clamp: false,
                mipmap_strategy: SpriteMipmapStrategy::Mean,
                alpha_cutoff_bias: 0.0,
            }
        );

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sprite_image_records_gui_tile_metadata() {
        let dir = unique_temp_dir("gui-tile-image");
        let path = dir.join("panel.png");
        write_test_png(&path, 8, 8);
        write_json(
            &dir.join("panel.png.mcmeta"),
            r#"{
              "gui": {
                "scaling": {
                  "type": "tile",
                  "width": 4,
                  "height": 5
                }
              }
            }"#,
        );

        let image = SpriteImage::from_png_file("minecraft:panel", &path).unwrap();

        assert_eq!(
            image.gui_metadata,
            SpriteGuiMetadata {
                scaling: SpriteGuiScaling::Tile {
                    width: 4,
                    height: 5,
                },
            }
        );

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sprite_image_interpolates_animation_frame_rgba() {
        let mut image =
            SpriteImage::new("minecraft:block/interpolated", 1, 1, vec![0, 20, 100, 255]).unwrap();
        image.animation = Some(SpriteAnimation {
            frame_count: 2,
            default_frame_time: 1,
            interpolate: true,
            frames: vec![
                SpriteAnimationFrame { index: 0, time: 4 },
                SpriteAnimationFrame { index: 1, time: 4 },
            ],
        });
        image.animation_frames_rgba = vec![vec![0, 20, 100, 255], vec![100, 60, 0, 127]];

        assert_eq!(
            image.frame_rgba_at_tick(0).unwrap().as_ref(),
            [0, 20, 100, 255]
        );
        assert_eq!(
            image.frame_rgba_at_tick(1).unwrap().as_ref(),
            [25, 30, 75, 223]
        );
        assert_eq!(
            image.frame_rgba_at_tick(2).unwrap().as_ref(),
            [50, 40, 50, 191]
        );
        assert_eq!(
            image.frame_rgba_at_tick(4).unwrap().as_ref(),
            [100, 60, 0, 127]
        );
    }

    #[test]
    fn sprite_image_records_alpha_transparency() {
        let opaque = SpriteImage::new("test:opaque", 1, 1, vec![1, 2, 3, 255]).unwrap();
        assert!(!opaque.transparency.has_transparent);
        assert!(!opaque.transparency.has_translucent);
        assert_eq!(opaque.frame_rgba(0), Some([1, 2, 3, 255].as_slice()));
        assert_eq!(opaque.frame_rgba(1), None);

        let transparent = SpriteImage::new("test:transparent", 1, 1, vec![1, 2, 3, 0]).unwrap();
        assert!(transparent.transparency.has_transparent);
        assert!(!transparent.transparency.has_translucent);

        let translucent = SpriteImage::new("test:translucent", 1, 1, vec![1, 2, 3, 127]).unwrap();
        assert!(!translucent.transparency.has_transparent);
        assert!(translucent.transparency.has_translucent);
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

    #[test]
    fn sprite_source_rejects_out_of_range_animation_frame() {
        let dir = unique_temp_dir("animation-invalid-frame");
        let path = dir.join("bad.png");
        write_test_png(&path, 4, 8);
        write_json(
            &dir.join("bad.png.mcmeta"),
            r#"{
              "animation": {
                "height": 4,
                "frames": [0, 2]
              }
            }"#,
        );

        let err = SpriteSource::from_png_file("minecraft:bad", &path).unwrap_err();
        assert!(err.to_string().contains("exceeds frame count"));

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sprite_source_rejects_empty_animation_frames() {
        let dir = unique_temp_dir("animation-empty-frames");
        let path = dir.join("bad.png");
        write_test_png(&path, 4, 8);
        write_json(
            &dir.join("bad.png.mcmeta"),
            r#"{
              "animation": {
                "height": 4,
                "frames": []
              }
            }"#,
        );

        let err = SpriteSource::from_png_file("minecraft:bad", &path).unwrap_err();
        assert!(err.to_string().contains("must contain at least one frame"));

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
