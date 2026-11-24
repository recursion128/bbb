use std::{
    fmt,
    io::{Cursor, ErrorKind, Read},
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use anyhow::{bail, Context, Result};
use image::{DynamicImage, ImageDecoder, ImageError, ImageFormat, ImageReader, RgbaImage};

pub(crate) const MAX_RESOURCE_PNG_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const MAX_RESOURCE_PNG_DIMENSION: u32 = 8_192;
pub(crate) const MAX_RESOURCE_PNG_PIXELS: u64 = 16 * 1024 * 1024;
pub(crate) const MAX_RESOURCE_PNG_DECODE_BYTES: u64 = 128 * 1024 * 1024;
pub(crate) const MAX_ATLAS_RGBA_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_RESOURCE_IMAGE_PROCESS_RGBA_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Debug)]
pub struct ResourcePngContentInvalid {
    source: anyhow::Error,
}

impl fmt::Display for ResourcePngContentInvalid {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.source.fmt(formatter)
    }
}

impl std::error::Error for ResourcePngContentInvalid {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

fn png_content_invalid(source: anyhow::Error) -> anyhow::Error {
    ResourcePngContentInvalid { source }.into()
}

const PNG_IEND_EMPTY_CRC: u32 = 0xae42_6082;

fn is_png_codec_content_error(error: &ImageError) -> bool {
    matches!(error, ImageError::Decoding(_) | ImageError::Unsupported(_))
        || matches!(error, ImageError::IoError(source) if source.kind() == ErrorKind::UnexpectedEof)
}

fn classify_png_codec_error(error: ImageError) -> anyhow::Error {
    if is_png_codec_content_error(&error) {
        // The codec only reads an admitted in-memory snapshot.
        png_content_invalid(error.into())
    } else {
        error.into()
    }
}

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

    pub(crate) fn reserve(&self, rgba_bytes: u64, label: &str) -> Result<()> {
        let next = self
            .charged_rgba_bytes
            .try_update(Ordering::AcqRel, Ordering::Acquire, |charged| {
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
        self.reserve_raw_bytes(rgba_bytes, label)
    }

    pub(crate) fn reserve_raw_bytes(&mut self, rgba_bytes: u64, label: &str) -> Result<()> {
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
    let bytes = read_png_file_snapshot(path, label)?;
    read_png_snapshot_dimensions(bytes, path, label, budget)
}

pub(crate) fn read_png_resource_snapshot(
    resource: &crate::PackResource,
    label: &str,
) -> Result<Vec<u8>> {
    match resource.archive_source() {
        Some(source) => read_png_zip_snapshot(source, &resource.path, label),
        None => read_png_file_snapshot(&resource.path, label),
    }
}

#[cfg(test)]
pub(crate) fn read_png_resource_dimensions_with_budget(
    resource: &crate::PackResource,
    label: &str,
    budget: &mut PngDecodeBudget,
) -> Result<(u32, u32)> {
    let bytes = read_png_resource_snapshot(resource, label)?;
    read_png_snapshot_dimensions(bytes, &resource.path, label, budget)
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
    let bytes = read_png_file_snapshot(path, label)?;
    decode_png_snapshot_rgba(bytes, path, label, budget)
}

pub(crate) fn decode_png_resource_rgba_with_budget(
    resource: &crate::PackResource,
    label: &str,
    budget: &mut PngDecodeBudget,
) -> Result<RgbaImage> {
    match resource.archive_source() {
        Some(source) => decode_png_zip_rgba_with_budget(source, &resource.path, label, budget),
        None => decode_png_file_rgba_with_budget(&resource.path, label, budget),
    }
}

pub(crate) fn decode_png_root_resource_rgba_with_budget(
    resource: &crate::PackRootResource,
    label: &str,
    budget: &mut PngDecodeBudget,
) -> Result<RgbaImage> {
    match resource.archive_source() {
        Some(source) => decode_png_zip_rgba_with_budget(source, &resource.path, label, budget),
        None => decode_png_file_rgba_with_budget(&resource.path, label, budget),
    }
}

fn decode_png_zip_rgba_with_budget(
    source: &crate::zip_pack::ZipEntrySource,
    display_path: &Path,
    label: &str,
    budget: &mut PngDecodeBudget,
) -> Result<RgbaImage> {
    let bytes = read_png_zip_snapshot(source, display_path, label)?;
    decode_png_snapshot_rgba(bytes, display_path, label, budget)
}

fn read_png_zip_snapshot(
    source: &crate::zip_pack::ZipEntrySource,
    display_path: &Path,
    label: &str,
) -> Result<Vec<u8>> {
    source.with_entry(|entry| {
        let size = entry.size();
        read_png_snapshot(entry, size, &format!("{label} {}", display_path.display()))
    })
}

fn read_png_file_snapshot(path: &Path, label: &str) -> Result<Vec<u8>> {
    let (mut file, file_bytes) = crate::virtual_store::open_resource_file(path)
        .with_context(|| format!("open PNG {label} {}", path.display()))?;
    let bytes = read_png_snapshot(
        &mut file,
        file_bytes,
        &format!("{label} {}", path.display()),
    )?;
    if crate::virtual_store::resource_file_len(path)
        .with_context(|| format!("recheck PNG metadata for {label} {}", path.display()))?
        != file_bytes
    {
        bail!("PNG {label} {} changed while being read", path.display());
    }
    Ok(bytes)
}

#[cfg(test)]
type AdmittedPngReader = (ImageReader<Cursor<Vec<u8>>>, (u32, u32));

#[cfg(test)]
fn png_file_reader(
    path: &Path,
    label: &str,
    budget: &mut PngDecodeBudget,
) -> Result<AdmittedPngReader> {
    let bytes = read_png_file_snapshot(path, label)?;
    let dimensions = admit_png_snapshot(&bytes, label, budget)?;
    let mut reader = ImageReader::with_format(Cursor::new(bytes), ImageFormat::Png);
    reader.limits(resource_png_limits());
    Ok((reader, dimensions))
}

fn read_png_snapshot_dimensions(
    bytes: Vec<u8>,
    display_path: &Path,
    label: &str,
    budget: &mut PngDecodeBudget,
) -> Result<(u32, u32)> {
    let header_dimensions = admit_png_snapshot(&bytes, label, budget)?;
    verify_admitted_png_snapshot_dimensions(&bytes, header_dimensions, display_path, label)
}

pub(crate) fn verify_admitted_png_snapshot_dimensions(
    bytes: &[u8],
    header_dimensions: (u32, u32),
    display_path: &Path,
    label: &str,
) -> Result<(u32, u32)> {
    let mut reader = ImageReader::with_format(Cursor::new(bytes), ImageFormat::Png);
    reader.limits(resource_png_limits());
    let decoder = match reader.into_decoder() {
        Ok(decoder) => decoder,
        Err(error) => {
            if is_png_codec_content_error(&error) {
                validate_png_chunk_crcs(bytes, label)?;
            }
            return Err(classify_png_codec_error(error)).with_context(|| {
                format!("read PNG header for {label} {}", display_path.display())
            });
        }
    };
    if decoder.dimensions() != header_dimensions {
        bail!(
            "PNG {label} {} changed while being read",
            display_path.display()
        );
    }
    validate_decoder_bytes(&decoder, label)?;
    Ok(header_dimensions)
}

fn decode_png_snapshot_rgba(
    bytes: Vec<u8>,
    display_path: &Path,
    label: &str,
    budget: &mut PngDecodeBudget,
) -> Result<RgbaImage> {
    let header_dimensions = admit_png_snapshot(&bytes, label, budget)?;
    decode_admitted_png_snapshot_rgba(&bytes, header_dimensions, display_path, label)
}

pub(crate) fn decode_admitted_png_snapshot_rgba(
    bytes: &[u8],
    header_dimensions: (u32, u32),
    display_path: &Path,
    label: &str,
) -> Result<RgbaImage> {
    let mut reader = ImageReader::with_format(Cursor::new(bytes), ImageFormat::Png);
    reader.limits(resource_png_limits());
    let decoder = match reader.into_decoder() {
        Ok(decoder) => decoder,
        Err(error) => {
            if is_png_codec_content_error(&error) {
                validate_png_chunk_crcs(bytes, label)?;
            }
            return Err(classify_png_codec_error(error)).with_context(|| {
                format!("read PNG header for {label} {}", display_path.display())
            });
        }
    };
    if decoder.dimensions() != header_dimensions {
        bail!(
            "PNG {label} {} changed while being read",
            display_path.display()
        );
    }
    validate_decoder_bytes(&decoder, label)?;
    let image = match DynamicImage::from_decoder(decoder) {
        Ok(image) => image,
        Err(error) => {
            if is_png_codec_content_error(&error) {
                validate_png_chunk_crcs(bytes, label)?;
            }
            return Err(classify_png_codec_error(error))
                .with_context(|| format!("decode PNG {label} {}", display_path.display()));
        }
    };
    Ok(image.into_rgba8())
}

pub(crate) fn admit_png_snapshot(
    bytes: &[u8],
    label: &str,
    budget: &mut PngDecodeBudget,
) -> Result<(u32, u32)> {
    let dimensions = png_header_dimensions(bytes, label);
    if let Ok((width, height)) = dimensions {
        budget.reserve_rgba(width, height, label)?;
    }
    match dimensions {
        Ok(dimensions) => {
            // Verify chunk framing plus CRCs that `image`'s PNG decoder skips
            // (ancillary chunks and post-IDAT chunks including IEND) without
            // hashing contiguous IDAT payloads a second time.
            validate_png_framing_and_unverified_crcs(bytes, label)?;
            Ok(dimensions)
        }
        Err(header_error) => {
            // Check every locatable CRC before returning a content error from the header.
            validate_png_chunk_crcs(bytes, label)?;
            Err(header_error)
        }
    }
}

fn read_png_snapshot(reader: impl Read, file_bytes: u64, label: &str) -> Result<Vec<u8>> {
    if file_bytes > MAX_RESOURCE_PNG_FILE_BYTES {
        bail!(
            "PNG {label} is {file_bytes} bytes, exceeding the {} byte limit",
            MAX_RESOURCE_PNG_FILE_BYTES
        );
    }
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(usize::try_from(file_bytes).context("PNG file size overflow")?)?;
    reader
        .take(MAX_RESOURCE_PNG_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .with_context(|| format!("read PNG snapshot for {label}"))?;
    if bytes.len() as u64 != file_bytes {
        bail!(
            "PNG {label} changed while being read (expected {file_bytes} bytes, read {})",
            bytes.len()
        );
    }
    Ok(bytes)
}

fn png_header_dimensions(bytes: &[u8], label: &str) -> Result<(u32, u32)> {
    if bytes.len() < 24
        || bytes[..8] != *b"\x89PNG\r\n\x1a\n"
        || bytes[8..12] != 13_u32.to_be_bytes()
        || bytes[12..16] != *b"IHDR"
    {
        return Err(png_content_invalid(anyhow::anyhow!(
            "{label} is not a PNG with an IHDR"
        )));
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().expect("fixed PNG width slice"));
    let height = u32::from_be_bytes(bytes[20..24].try_into().expect("fixed PNG height slice"));
    if width == 0 || height == 0 {
        return Err(png_content_invalid(anyhow::anyhow!(
            "resource PNG {label} must not be empty"
        )));
    }
    Ok((width, height))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum IdatSequenceState {
    Before,
    In,
    After,
}

fn validate_png_framing_and_unverified_crcs(bytes: &[u8], label: &str) -> Result<()> {
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(png_content_invalid(anyhow::anyhow!("{label} is not a PNG")));
    }
    let mut offset = 8_usize;
    let mut idat_state = IdatSequenceState::Before;
    while let Some(header) = bytes.get(offset..).and_then(|tail| tail.get(..8)) {
        let length = u32::from_be_bytes(header[..4].try_into().expect("fixed PNG length slice"));
        let end = offset
            .checked_add(12)
            .and_then(|offset| offset.checked_add(length as usize));
        let Some(end) = end.filter(|&end| end <= bytes.len()) else {
            validate_png_chunk_crcs(bytes, label)?;
            return Err(png_content_invalid(anyhow::anyhow!(
                "PNG {label} has a truncated chunk"
            )));
        };
        let kind = &header[4..8];
        let decoder_verifies_crc = match kind {
            b"IDAT" => match idat_state {
                IdatSequenceState::Before => {
                    idat_state = IdatSequenceState::In;
                    true
                }
                IdatSequenceState::In => true,
                IdatSequenceState::After => false,
            },
            b"IHDR" | b"PLTE" if idat_state == IdatSequenceState::Before => true,
            _ => {
                if idat_state == IdatSequenceState::In {
                    idat_state = IdatSequenceState::After;
                }
                false
            }
        };
        if !decoder_verifies_crc {
            let expected =
                u32::from_be_bytes(bytes[end - 4..end].try_into().expect("fixed PNG CRC slice"));
            let valid = if kind == b"IEND" && length == 0 {
                expected == PNG_IEND_EMPTY_CRC
            } else {
                crc32fast::hash(&bytes[offset + 4..end - 4]) == expected
            };
            if !valid {
                bail!("PNG {label} chunk {:?} has an invalid CRC", kind);
            }
        }
        if kind == b"IEND" {
            return Ok(());
        }
        offset = end;
    }
    validate_png_chunk_crcs(bytes, label)?;
    Err(png_content_invalid(anyhow::anyhow!(
        "PNG {label} is truncated before IEND"
    )))
}

fn validate_png_chunk_crcs(bytes: &[u8], label: &str) -> Result<()> {
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(png_content_invalid(anyhow::anyhow!("{label} is not a PNG")));
    }
    let mut offset = 8_usize;
    while let Some(header) = bytes.get(offset..).and_then(|tail| tail.get(..8)) {
        let length = u32::from_be_bytes(header[..4].try_into().expect("fixed PNG length slice"));
        let end = offset
            .checked_add(12)
            .and_then(|offset| offset.checked_add(length as usize));
        let Some(end) = end.filter(|&end| end <= bytes.len()) else {
            return Err(png_content_invalid(anyhow::anyhow!(
                "PNG {label} has a truncated chunk"
            )));
        };
        let expected =
            u32::from_be_bytes(bytes[end - 4..end].try_into().expect("fixed PNG CRC slice"));
        if crc32fast::hash(&bytes[offset + 4..end - 4]) != expected {
            bail!("PNG {label} chunk {:?} has an invalid CRC", &header[4..8]);
        }
        if header[4..8] == *b"IEND" {
            return Ok(());
        }
        offset = end;
    }
    Err(png_content_invalid(anyhow::anyhow!(
        "PNG {label} is truncated before IEND"
    )))
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
    fn malformed_stable_png_content_has_the_narrow_marker() {
        let complete = png_with_ihdr_dimensions(1, 1);
        let mut bad_signature = complete.clone();
        bad_signature[0] = 0;
        let mut non_png_with_bad_crc = bad_signature.clone();
        let last = non_png_with_bad_crc.len() - 1;
        non_png_with_bad_crc[last] ^= 1;
        let mut bad_ihdr = b"\x89PNG\r\n\x1a\n".to_vec();
        append_chunk(&mut bad_ihdr, b"IHDR", &[0; 12]);
        append_chunk(&mut bad_ihdr, b"IEND", &[]);
        let mut bad_content = complete.clone();
        bad_content[25] = 7;
        replace_crc(&mut bad_content, 8);
        let mut bad_idat = rgba_png();
        let offset = chunk_offset(&bad_idat, b"IDAT");
        bad_idat[offset + 8] ^= 0xff;
        replace_crc(&mut bad_idat, offset);
        for (label, bytes) in [
            ("empty", Vec::new()),
            ("short header", complete[..23].to_vec()),
            ("truncated chunk", complete[..complete.len() - 1].to_vec()),
            ("bad signature", bad_signature),
            ("non PNG with chunk shaped bytes", non_png_with_bad_crc),
            ("bad IHDR", bad_ihdr),
            ("zero width", png_with_ihdr_dimensions(0, 1)),
            ("zero height", png_with_ihdr_dimensions(1, 0)),
            ("invalid color type", bad_content),
            ("invalid deflate stream with correct CRC", bad_idat),
            ("missing IDAT", complete),
        ] {
            let path = temp_path(label);
            std::fs::write(&path, bytes).unwrap();
            let error = decode_png_file_rgba(&path, label).unwrap_err();
            assert!(
                error.is::<ResourcePngContentInvalid>(),
                "{label}: {error:#}"
            );
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn every_png_chunk_crc_remains_a_hard_failure() {
        let complete = rgba_png();
        let mut with_ancillary = complete[..complete.len() - 12].to_vec();
        append_chunk(&mut with_ancillary, b"teSt", b"ignored ancillary data");
        append_chunk(&mut with_ancillary, b"IEND", &[]);
        for kind in [b"IHDR", b"IDAT", b"teSt", b"IEND"] {
            let mut bytes = with_ancillary.clone();
            let offset = chunk_offset(&bytes, kind);
            let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
            bytes[offset + 8 + length] ^= 1;
            let path = temp_path("crc");
            std::fs::write(&path, bytes).unwrap();
            let error = decode_png_file_rgba(&path, "CRC fixture").unwrap_err();
            assert!(
                !error.is::<ResourcePngContentInvalid>(),
                "{kind:?}: {error:#}"
            );
            assert!(error.to_string().contains("invalid CRC"), "{error:#}");
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn locatable_later_crc_has_priority_over_invalid_png_content() {
        for prefix in ["IHDR", "zero", "codec", "IDAT"] {
            let mut bytes = rgba_png();
            match prefix {
                "IHDR" => {
                    bytes[12..16].copy_from_slice(b"NOPE");
                    replace_crc(&mut bytes, 8);
                }
                "zero" => {
                    bytes[16..20].copy_from_slice(&0_u32.to_be_bytes());
                    replace_crc(&mut bytes, 8);
                }
                "codec" => {
                    bytes[25] = 7;
                    replace_crc(&mut bytes, 8);
                }
                "IDAT" => {
                    let offset = chunk_offset(&bytes, b"IDAT");
                    bytes[offset + 8] ^= 0xff;
                    replace_crc(&mut bytes, offset);
                }
                _ => unreachable!(),
            }
            let last = bytes.len() - 1;
            bytes[last] ^= 1;
            let path = temp_path("content-before-crc");
            std::fs::write(&path, bytes).unwrap();
            let error = decode_png_file_rgba(&path, prefix).unwrap_err();
            assert!(
                !error.is::<ResourcePngContentInvalid>(),
                "{prefix}: {error:#}"
            );
            assert!(error.to_string().contains("invalid CRC"), "{error:#}");
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn secondary_idat_crc_and_bad_crc_before_truncation_remain_hard_failures() {
        let mut multi_idat = rgba_png();
        let iend_offset = chunk_offset(&multi_idat, b"IEND");
        let mut with_second_idat = multi_idat[..iend_offset].to_vec();
        append_chunk(&mut with_second_idat, b"IDAT", &[]);
        let second_idat_crc_byte = with_second_idat.len() - 1;
        with_second_idat[second_idat_crc_byte] ^= 1;
        append_chunk(&mut with_second_idat, b"IEND", &[]);
        let path = temp_path("second-idat-crc");
        std::fs::write(&path, &with_second_idat).unwrap();
        let error = decode_png_file_rgba(&path, "second IDAT CRC").unwrap_err();
        assert!(!error.is::<ResourcePngContentInvalid>(), "{error:#}");
        assert!(error.to_string().contains("invalid CRC"), "{error:#}");
        std::fs::remove_file(&path).unwrap();

        let ihdr_offset = chunk_offset(&multi_idat, b"IHDR");
        let ihdr_len =
            u32::from_be_bytes(multi_idat[ihdr_offset..ihdr_offset + 4].try_into().unwrap())
                as usize;
        multi_idat[ihdr_offset + 8 + ihdr_len] ^= 1;
        multi_idat.pop();
        let path = temp_path("bad-crc-truncated");
        std::fs::write(&path, &multi_idat).unwrap();
        let error = decode_png_file_rgba(&path, "bad CRC before truncation").unwrap_err();
        assert!(!error.is::<ResourcePngContentInvalid>(), "{error:#}");
        assert!(error.to_string().contains("invalid CRC"), "{error:#}");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn png_content_failure_keeps_dimension_admission_and_pixel_limits_hard() {
        let path = temp_path("content-admission");
        let mut bytes = png_with_ihdr_dimensions(2, 2);
        bytes[25] = 7;
        replace_crc(&mut bytes, 8);
        std::fs::write(&path, bytes).unwrap();
        let generation = ResourceImageGenerationBudget::new(16);
        let mut budget = PngDecodeBudget::single_image_in(Arc::clone(&generation));
        let error =
            decode_png_file_rgba_with_budget(&path, "invalid color", &mut budget).unwrap_err();
        assert!(error.is::<ResourcePngContentInvalid>(), "{error:#}");
        assert_eq!(budget.used_rgba_bytes, 16);
        assert_eq!(generation.observation().charged_rgba_bytes, 16);
        for (width, height, message) in [
            (MAX_RESOURCE_PNG_DIMENSION + 1, 1, "dimension limit"),
            (8192, 4096, "pixel limit"),
        ] {
            std::fs::write(&path, png_with_ihdr_dimensions(width, height)).unwrap();
            let error = decode_png_file_rgba(&path, "limits").unwrap_err();
            assert!(!error.is::<ResourcePngContentInvalid>(), "{error:#}");
            assert!(error.to_string().contains(message), "{error:#}");
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn png_codec_classification_keeps_limits_memory_and_non_cursor_io_hard() {
        use image::error::{LimitError, LimitErrorKind, ParameterError, ParameterErrorKind};

        for error in [
            ImageError::Limits(LimitError::from_kind(LimitErrorKind::InsufficientMemory)),
            ImageError::Limits(LimitError::from_kind(LimitErrorKind::DimensionError)),
            ImageError::IoError(std::io::Error::from(ErrorKind::OutOfMemory)),
            ImageError::IoError(std::io::Error::from(ErrorKind::PermissionDenied)),
            ImageError::Parameter(ParameterError::from_kind(
                ParameterErrorKind::DimensionMismatch,
            )),
        ] {
            let error = classify_png_codec_error(error);
            assert!(!error.is::<ResourcePngContentInvalid>(), "{error:#}");
        }
        let error = classify_png_codec_error(ImageError::IoError(std::io::Error::from(
            ErrorKind::UnexpectedEof,
        )));
        assert!(error.is::<ResourcePngContentInvalid>());
    }

    #[test]
    fn png_snapshot_preserves_real_io_failures_and_length_changes_as_hard() {
        struct FailingReader;
        impl Read for FailingReader {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(ErrorKind::UnexpectedEof.into())
            }
        }
        let error = read_png_snapshot(FailingReader, 1, "underlying read").unwrap_err();
        assert!(!error.is::<ResourcePngContentInvalid>());
        assert_eq!(
            error.downcast_ref::<std::io::Error>().unwrap().kind(),
            ErrorKind::UnexpectedEof
        );
        for (bytes, expected) in [(b"abc".as_slice(), 2), (b"a".as_slice(), 2)] {
            let error = read_png_snapshot(bytes, expected, "changed length").unwrap_err();
            assert!(!error.is::<ResourcePngContentInvalid>());
            assert!(error.to_string().contains("changed while being read"));
        }
        let path = temp_path("missing-or-directory");
        let missing = decode_png_file_rgba(&path, "missing").unwrap_err();
        assert!(!missing.is::<ResourcePngContentInvalid>());
        assert_eq!(
            missing.downcast_ref::<std::io::Error>().unwrap().kind(),
            ErrorKind::NotFound
        );
        std::fs::create_dir(&path).unwrap();
        let directory = decode_png_file_rgba(&path, "directory").unwrap_err();
        assert!(!directory.is::<ResourcePngContentInvalid>());
        assert!(directory.downcast_ref::<std::io::Error>().is_some());
        std::fs::remove_dir(path).unwrap();
    }

    #[test]
    fn png_snapshot_caps_growth_and_decoder_never_rereads_the_source() {
        struct GrowingReader(u64);
        impl Read for GrowingReader {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                buffer.fill(0);
                self.0 += buffer.len() as u64;
                Ok(buffer.len())
            }
        }
        let mut growing = GrowingReader(0);
        let error = read_png_snapshot(&mut growing, 1, "growing source").unwrap_err();
        assert!(!error.is::<ResourcePngContentInvalid>());
        assert_eq!(growing.0, MAX_RESOURCE_PNG_FILE_BYTES + 1);

        let path = temp_path("stable-snapshot");
        std::fs::write(&path, rgba_png()).unwrap();
        let (reader, _) =
            png_file_reader(&path, "snapshot", &mut PngDecodeBudget::single_image()).unwrap();
        std::fs::write(&path, b"replaced after snapshot").unwrap();
        let image = DynamicImage::from_decoder(reader.into_decoder().unwrap())
            .unwrap()
            .into_rgba8();
        assert_eq!(image.get_pixel(0, 0).0, [1, 2, 3, 255]);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn admitted_png_keeps_rgba_palette_sixteen_bit_and_trailing_byte_behavior() {
        use image::ImageEncoder;

        let mut palette = b"\x89PNG\r\n\x1a\n".to_vec();
        let header = [0, 0, 0, 1, 0, 0, 0, 1, 8, 3, 0, 0, 0];
        append_chunk(&mut palette, b"IHDR", &header);
        append_chunk(&mut palette, b"PLTE", &[10, 20, 30, 40, 50, 60]);
        append_chunk(
            &mut palette,
            b"IDAT",
            &[
                0x78, 0x01, 0x01, 0x02, 0x00, 0xfd, 0xff, 0x00, 0x01, 0x00, 0x03, 0x00, 0x02,
            ],
        );
        append_chunk(&mut palette, b"IEND", &[]);
        let mut sixteen = Vec::new();
        let samples: Vec<u8> = [0_u16, 65535, 0, 65535]
            .into_iter()
            .flat_map(u16::to_ne_bytes)
            .collect();
        image::codecs::png::PngEncoder::new(&mut sixteen)
            .write_image(&samples, 1, 1, image::ExtendedColorType::Rgba16)
            .unwrap();
        let mut trailing = rgba_png();
        trailing.extend_from_slice(b"trailing bytes after IEND");
        for (bytes, expected) in [
            (rgba_png(), [1, 2, 3, 255]),
            (palette, [40, 50, 60, 255]),
            (sixteen, [0, 255, 0, 255]),
            (trailing, [1, 2, 3, 255]),
        ] {
            let path = temp_path("png-color-conversion");
            std::fs::write(&path, bytes).unwrap();
            let image = decode_png_file_rgba(&path, "color conversion").unwrap();
            assert_eq!(image.get_pixel(0, 0).0, expected);
            std::fs::remove_file(path).unwrap();
        }
    }

    fn rgba_png() -> Vec<u8> {
        use image::ImageEncoder;
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(&[1, 2, 3, 255], 1, 1, image::ExtendedColorType::Rgba8)
            .unwrap();
        bytes
    }

    fn chunk_offset(bytes: &[u8], kind: &[u8; 4]) -> usize {
        let mut offset = 8;
        loop {
            if &bytes[offset + 4..offset + 8] == kind {
                return offset;
            }
            offset +=
                12 + u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        }
    }

    fn replace_crc(bytes: &mut [u8], offset: usize) {
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let crc = crc32fast::hash(&bytes[offset + 4..offset + 8 + length]);
        bytes[offset + 8 + length..offset + 12 + length].copy_from_slice(&crc.to_be_bytes());
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
            "bbb-client-assets-{label}-{}-{nanos}.png",
            std::process::id()
        ))
    }
}
