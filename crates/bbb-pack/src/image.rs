use std::{
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use anyhow::{bail, Context, Result};
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, RgbaImage};

pub(crate) const MAX_RESOURCE_PNG_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const MAX_RESOURCE_PNG_DIMENSION: u32 = 8_192;
pub(crate) const MAX_RESOURCE_PNG_PIXELS: u64 = 16 * 1024 * 1024;
pub(crate) const MAX_RESOURCE_PNG_DECODE_BYTES: u64 = 128 * 1024 * 1024;
pub(crate) const MAX_ATLAS_RGBA_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_RESOURCE_IMAGE_PROCESS_RGBA_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResourceImageMemoryObservation {
    pub max_rgba_bytes: u64,
    pub charged_rgba_bytes: u64,
    pub peak_charged_rgba_bytes: u64,
}

#[derive(Debug)]
pub(crate) struct ResourceImageGenerationBudget {
    max_rgba_bytes: u64,
    charged_rgba_bytes: AtomicU64,
    peak_charged_rgba_bytes: AtomicU64,
}

impl ResourceImageGenerationBudget {
    pub(crate) fn new(max_rgba_bytes: u64) -> Arc<Self> {
        Arc::new(Self {
            max_rgba_bytes,
            charged_rgba_bytes: AtomicU64::new(0),
            peak_charged_rgba_bytes: AtomicU64::new(0),
        })
    }

    fn reserve(&self, rgba_bytes: u64, label: &str) -> Result<()> {
        let next = self
            .charged_rgba_bytes
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |charged| {
                charged
                    .checked_add(rgba_bytes)
                    .filter(|next| *next <= self.max_rgba_bytes)
            })
            .map(|previous| previous + rgba_bytes)
            .map_err(|charged| {
                anyhow::anyhow!(
                    "resource image generation would use more than {} RGBA bytes while loading {label} (currently charged {charged}, requested {rgba_bytes})",
                    self.max_rgba_bytes
                )
            })?;
        self.peak_charged_rgba_bytes
            .fetch_max(next, Ordering::Relaxed);
        Ok(())
    }

    pub(crate) fn observation(&self) -> ResourceImageMemoryObservation {
        ResourceImageMemoryObservation {
            max_rgba_bytes: self.max_rgba_bytes,
            charged_rgba_bytes: self.charged_rgba_bytes.load(Ordering::Acquire),
            peak_charged_rgba_bytes: self.peak_charged_rgba_bytes.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PngDecodeBudget {
    max_rgba_bytes: u64,
    used_rgba_bytes: u64,
    generation: Arc<ResourceImageGenerationBudget>,
}

impl PngDecodeBudget {
    pub(crate) fn single_image() -> Self {
        Self::new(
            MAX_RESOURCE_PNG_PIXELS * 4,
            ResourceImageGenerationBudget::new(MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES),
        )
    }

    #[cfg(test)]
    pub(crate) fn atlas() -> Self {
        Self::new(
            MAX_ATLAS_RGBA_BYTES,
            ResourceImageGenerationBudget::new(MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES),
        )
    }

    pub(crate) fn single_image_in(generation: Arc<ResourceImageGenerationBudget>) -> Self {
        Self::new(MAX_RESOURCE_PNG_PIXELS * 4, generation)
    }

    pub(crate) fn atlas_in(generation: Arc<ResourceImageGenerationBudget>) -> Self {
        Self::new(MAX_ATLAS_RGBA_BYTES, generation)
    }

    fn new(max_rgba_bytes: u64, generation: Arc<ResourceImageGenerationBudget>) -> Self {
        Self {
            max_rgba_bytes,
            used_rgba_bytes: 0,
            generation,
        }
    }

    pub(crate) fn reserve_rgba(&mut self, width: u32, height: u32, label: &str) -> Result<()> {
        validate_resource_png_dimensions(width, height, label)?;
        let rgba_bytes = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| anyhow::anyhow!("{label} RGBA size overflow"))?;
        let next = self
            .used_rgba_bytes
            .checked_add(rgba_bytes)
            .ok_or_else(|| anyhow::anyhow!("resource PNG aggregate RGBA size overflow"))?;
        if next > self.max_rgba_bytes {
            bail!(
                "resource PNG aggregate would use {next} RGBA bytes, exceeding the {} byte limit while loading {label}",
                self.max_rgba_bytes
            );
        }
        self.generation.reserve(rgba_bytes, label)?;
        self.used_rgba_bytes = next;
        Ok(())
    }
}

pub(crate) fn read_png_file_dimensions_with_budget(
    path: &Path,
    label: &str,
    budget: &mut PngDecodeBudget,
) -> Result<(u32, u32)> {
    let decoder = png_file_decoder(path, label, budget)?;
    let (width, height) = decoder.dimensions();
    validate_decoder_bytes(&decoder, label)?;
    Ok((width, height))
}

pub(crate) fn decode_png_file_rgba(path: &Path, label: &str) -> Result<RgbaImage> {
    let mut budget = PngDecodeBudget::single_image();
    decode_png_file_rgba_with_budget(path, label, &mut budget)
}

pub(crate) fn decode_png_file_rgba_with_budget(
    path: &Path,
    label: &str,
    budget: &mut PngDecodeBudget,
) -> Result<RgbaImage> {
    let decoder = png_file_decoder(path, label, budget)?;
    validate_decoder_bytes(&decoder, label)?;
    DynamicImage::from_decoder(decoder)
        .with_context(|| format!("decode PNG {label} {}", path.display()))
        .map(DynamicImage::into_rgba8)
}

fn png_file_decoder(
    path: &Path,
    label: &str,
    budget: &mut PngDecodeBudget,
) -> Result<impl ImageDecoder> {
    let (reader, header_dimensions) = png_file_reader(path, label, budget)?;
    let decoder = reader
        .into_decoder()
        .with_context(|| format!("read PNG header for {label} {}", path.display()))?;
    if decoder.dimensions() != header_dimensions {
        bail!("PNG {label} {} changed while being read", path.display());
    }
    Ok(decoder)
}

fn png_file_reader(
    path: &Path,
    label: &str,
    budget: &mut PngDecodeBudget,
) -> Result<(ImageReader<BufReader<File>>, (u32, u32))> {
    let file = File::open(path).with_context(|| format!("open PNG {label} {}", path.display()))?;
    let file_bytes = file
        .metadata()
        .with_context(|| format!("read PNG metadata for {label} {}", path.display()))?
        .len();
    if file_bytes > MAX_RESOURCE_PNG_FILE_BYTES {
        bail!(
            "PNG {label} {} is {file_bytes} bytes, exceeding the {} byte limit",
            path.display(),
            MAX_RESOURCE_PNG_FILE_BYTES
        );
    }

    let mut file = BufReader::new(file);
    let mut header = [0_u8; 24];
    file.read_exact(&mut header)
        .with_context(|| format!("read PNG IHDR for {label} {}", path.display()))?;
    if header[..8] != *b"\x89PNG\r\n\x1a\n"
        || header[8..12] != 13_u32.to_be_bytes()
        || header[12..16] != *b"IHDR"
    {
        bail!("{label} {} is not a PNG", path.display());
    }
    let width = u32::from_be_bytes(header[16..20].try_into().expect("fixed PNG width slice"));
    let height = u32::from_be_bytes(header[20..24].try_into().expect("fixed PNG height slice"));
    budget.reserve_rgba(width, height, label)?;
    file.seek(SeekFrom::Start(0))
        .with_context(|| format!("rewind PNG {label} {}", path.display()))?;

    let mut reader = ImageReader::with_format(file, ImageFormat::Png);
    reader.limits(resource_png_limits());
    Ok((reader, (width, height)))
}

fn resource_png_limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_RESOURCE_PNG_DIMENSION);
    limits.max_image_height = Some(MAX_RESOURCE_PNG_DIMENSION);
    limits.max_alloc = Some(MAX_RESOURCE_PNG_DECODE_BYTES);
    limits
}

fn validate_resource_png_dimensions(width: u32, height: u32, label: &str) -> Result<()> {
    if width == 0 || height == 0 {
        bail!("resource PNG {label} must not be empty");
    }
    if width > MAX_RESOURCE_PNG_DIMENSION || height > MAX_RESOURCE_PNG_DIMENSION {
        bail!(
            "resource PNG {label} is {width}x{height}, exceeding the {} pixel dimension limit",
            MAX_RESOURCE_PNG_DIMENSION
        );
    }
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or_else(|| anyhow::anyhow!("resource PNG {label} pixel count overflow"))?;
    if pixels > MAX_RESOURCE_PNG_PIXELS {
        bail!(
            "resource PNG {label} has {pixels} pixels, exceeding the {} pixel limit",
            MAX_RESOURCE_PNG_PIXELS
        );
    }
    Ok(())
}

fn validate_decoder_bytes(decoder: &impl ImageDecoder, label: &str) -> Result<()> {
    let decoded_bytes = decoder.total_bytes();
    if decoded_bytes > MAX_RESOURCE_PNG_DECODE_BYTES {
        bail!(
            "resource PNG {label} requires {decoded_bytes} decoded bytes, exceeding the {} byte limit",
            MAX_RESOURCE_PNG_DECODE_BYTES
        );
    }
    Ok(())
}

pub(crate) fn rgba_offset(width: u32, x: u32, y: u32) -> Result<usize> {
    let pixel = y
        .checked_mul(width)
        .and_then(|row| row.checked_add(x))
        .ok_or_else(|| anyhow::anyhow!("RGBA offset overflow"))?;
    usize::try_from(pixel)
        .ok()
        .and_then(|pixel| pixel.checked_mul(4))
        .ok_or_else(|| anyhow::anyhow!("RGBA offset overflow"))
}

pub(crate) fn rgba_len(width: u32, height: u32) -> Result<usize> {
    let pixels = width
        .checked_mul(height)
        .ok_or_else(|| anyhow::anyhow!("RGBA image size overflow"))?;
    usize::try_from(pixels)
        .ok()
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow::anyhow!("RGBA image size overflow"))
}

#[cfg(test)]
mod tests {
    use std::{
        fs::OpenOptions,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    #[test]
    fn rejects_resource_png_file_size_before_decode() {
        let path = temp_path("oversized-file.png");
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .unwrap();
        file.set_len(MAX_RESOURCE_PNG_FILE_BYTES + 1).unwrap();

        let err = decode_png_file_rgba(&path, "oversized fixture")
            .unwrap_err()
            .to_string();
        assert!(err.contains("exceeding the 67108864 byte limit"), "{err}");

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_resource_png_dimensions_before_pixel_allocation() {
        let path = temp_path("oversized-dimensions.png");
        std::fs::write(
            &path,
            png_with_ihdr_dimensions(MAX_RESOURCE_PNG_DIMENSION + 1, 1),
        )
        .unwrap();

        let err = decode_png_file_rgba(&path, "dimension fixture")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("exceeding the 8192 pixel dimension limit"),
            "{err}"
        );

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_aggregate_rgba_before_decoding_another_image() {
        let path = temp_path("aggregate.png");
        let image = RgbaImage::from_pixel(2, 2, image::Rgba([1, 2, 3, 255]));
        image.save(&path).unwrap();
        let generation = ResourceImageGenerationBudget::new(16);
        let mut budget = PngDecodeBudget::new(16, Arc::clone(&generation));

        let first = decode_png_file_rgba_with_budget(&path, "first", &mut budget).unwrap();
        assert_eq!(first.dimensions(), (2, 2));
        let err = decode_png_file_rgba_with_budget(&path, "second", &mut budget)
            .unwrap_err()
            .to_string();
        assert!(err.contains("aggregate would use 32 RGBA bytes"), "{err}");
        assert_eq!(generation.observation().charged_rgba_bytes, 16);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn generation_budget_is_shared_across_independent_decode_scopes() {
        let path = temp_path("generation-aggregate.png");
        let image = RgbaImage::from_pixel(2, 2, image::Rgba([1, 2, 3, 255]));
        image.save(&path).unwrap();
        let generation = ResourceImageGenerationBudget::new(24);

        let first = decode_png_file_rgba_with_budget(
            &path,
            "first scope",
            &mut PngDecodeBudget::single_image_in(Arc::clone(&generation)),
        )
        .unwrap();
        assert_eq!(first.dimensions(), (2, 2));
        let err = decode_png_file_rgba_with_budget(
            &path,
            "second scope",
            &mut PngDecodeBudget::single_image_in(Arc::clone(&generation)),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("currently charged 16, requested 16"), "{err}");
        assert_eq!(
            generation.observation(),
            ResourceImageMemoryObservation {
                max_rgba_bytes: 24,
                charged_rgba_bytes: 16,
                peak_charged_rgba_bytes: 16,
            }
        );

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn failed_decode_keeps_its_generation_admission_charge() {
        let path = temp_path("failed-generation-decode.png");
        std::fs::write(&path, png_with_ihdr_dimensions(1, 1)).unwrap();
        let generation = ResourceImageGenerationBudget::new(4);

        let err = decode_png_file_rgba_with_budget(
            &path,
            "malformed payload",
            &mut PngDecodeBudget::single_image_in(Arc::clone(&generation)),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("malformed payload"), "{err}");
        assert_eq!(generation.observation().charged_rgba_bytes, 4);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn pack_png_consumers_do_not_bypass_the_bounded_decoder_owner() {
        for (path, source) in [
            ("atlas_sources.rs", include_str!("atlas_sources.rs")),
            ("colors.rs", include_str!("colors.rs")),
            ("panorama.rs", include_str!("panorama.rs")),
            ("resource_textures.rs", include_str!("resource_textures.rs")),
            ("sprites.rs", include_str!("sprites.rs")),
        ] {
            assert!(!source.contains("ImageReader"), "{path} bypasses image.rs");
            assert!(
                !source.contains("load_from_memory"),
                "{path} restored an unbounded memory decoder"
            );
            assert!(
                !source.contains("image::open"),
                "{path} restored an unbounded file decoder"
            );
        }
    }

    #[test]
    fn production_pack_consumers_use_the_generation_scoped_sprite_decoder() {
        for (path, source) in [
            (
                "bbb-item-model/item_runtime.rs",
                include_str!("../../bbb-item-model/src/item_runtime.rs"),
            ),
            (
                "bbb-item-model/item_runtime/profiles.rs",
                include_str!("../../bbb-item-model/src/item_runtime/profiles.rs"),
            ),
            (
                "bbb-item-model/font.rs",
                include_str!("../../bbb-item-model/src/font.rs"),
            ),
            (
                "bbb-native/hud_assets.rs",
                include_str!("../../bbb-native/src/hud_assets.rs"),
            ),
            (
                "bbb-native/sky_assets.rs",
                include_str!("../../bbb-native/src/sky_assets.rs"),
            ),
            (
                "bbb-native/entity_assets.rs",
                include_str!("../../bbb-native/src/entity_assets.rs"),
            ),
        ] {
            assert!(
                !source.contains("SpriteImage::from_png_file("),
                "{path} bypasses the resource-generation image budget"
            );
        }
        assert!(
            !include_str!("resource_textures.rs").contains("decode_png_file_rgba("),
            "standalone resource textures bypass the resource-generation image budget"
        );
    }

    fn png_with_ihdr_dimensions(width: u32, height: u32) -> Vec<u8> {
        let mut png = Vec::new();
        png.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&width.to_be_bytes());
        ihdr.extend_from_slice(&height.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
        append_chunk(&mut png, b"IHDR", &ihdr);
        append_chunk(&mut png, b"IEND", &[]);
        png
    }

    fn append_chunk(png: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        png.extend_from_slice(&(data.len() as u32).to_be_bytes());
        png.extend_from_slice(kind);
        png.extend_from_slice(data);
        let mut crc = 0xffff_ffff_u32;
        for byte in kind.iter().chain(data) {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xedb8_8320_u32 & (0_u32.wrapping_sub(crc & 1)));
            }
        }
        png.extend_from_slice(&(!crc).to_be_bytes());
    }

    fn temp_path(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "bbb-pack-{label}-{}-{nanos}.png",
            std::process::id()
        ))
    }
}
